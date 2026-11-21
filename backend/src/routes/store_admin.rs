//! Managing the store (docs/STORE.md): stores, items and discounts need Admin Access and
//! Store Management; a store's grantable usergroups need Elevated Control on top; receipts
//! and refunds need Admin Access and Store Finance.

use std::time::{Duration, Instant};

use actix_web::{HttpResponse, delete, get, post, put, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Server,
        cursor::{Cursor, Page},
        store::{
            CURRENCIES, DISPUTE_WON, DiscountDetails, Fulfillment, FulfillmentStatus, ItemDetails, ItemStore,
            MoneyOnRevoke, PaymentRefunds, PricedItem, Receipt, ReceiptFilter, ReceiptStatus, ReceiptView, Reward, StoreDetails,
            StoreDiscount, StoreItem, StoreSummary,
        },
    },
    notify,
    routes::{
        markdown::markdown,
        store::{sync_refunds, with_servers},
        validate::{bad, optional_url, required_text, text},
    },
    state::AppState,
    stripe::Stripe,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    // `receipts` before `{store_id}`, so it isn't matched as a store id.
    cfg.service(list_stores)
        .service(create_store)
        .service(list_receipts)
        .service(refund)
        .service(reinstate)
        .service(retry_fulfillment)
        .service(undelivered)
        .service(refund_undelivered)
        .service(get_store)
        .service(update_store)
        .service(set_grantable_usergroups)
        .service(create_item)
        .service(update_item)
        .service(archive_item)
        .service(create_discount)
        .service(update_discount)
        .service(delete_discount);
}

/// Like the other admin pages, managing the store needs Admin Access as well as the
/// specific permission.
const MANAGE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::STORE_MANAGEMENT);
const FINANCE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::STORE_FINANCE);
/// Changing which usergroups items may grant could hand out `superadmin`.
const GRANTABLE: Permissions = MANAGE.union(Permissions::ELEVATED_CONTROL);

const MAX_NAME: usize = 128;
const MAX_MESSAGE: usize = 2000;
const MAX_SHORT_DESCRIPTION: usize = 256;
const MAX_FULL_DESCRIPTION: usize = 20_000;
/// Minor units: ten million dollars is plenty.
const MAX_PRICE: i64 = 1_000_000_000;
const MAX_STOCK: i32 = 1_000_000;
const MAX_PER_USER: i32 = 1000;
const MAX_REWARDS: usize = 20;
const MAX_USERGROUP: usize = 64;
const MAX_USERGROUPS: usize = 50;
const MAX_WEAPON_CLASS: usize = 128;
const MAX_MONEY: i64 = 1_000_000_000_000;
/// A granted usergroup may last at most ten years.
const MAX_DURATION_SECONDS: i64 = 10 * 365 * 24 * 60 * 60;
/// Receipts per page of the finance list.
const DEFAULT_RECEIPTS: i64 = 100;
const MAX_RECEIPTS: i64 = 500;

/// Every store, enabled or not, with its server. Requires Admin Access and Store Management
/// or Store Finance (finance needs the list for its filter).
#[get("/admin/store")]
async fn list_stores(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    let permissions = user.require(&state.db, Permissions::ADMIN_ACCESS).await?;
    if !permissions.contains(Permissions::STORE_MANAGEMENT) && !permissions.contains(Permissions::STORE_FINANCE) {
        return Err(ApiError::Forbidden(format!("{} or {}", Permissions::STORE_MANAGEMENT.name(), Permissions::STORE_FINANCE.name())));
    }
    let (stores, servers) = tokio::try_join!(ItemStore::list(&state.db), Server::list(&state.db))?;
    Ok(HttpResponse::Ok().json(with_servers(stores, servers)))
}

#[derive(Deserialize)]
struct StoreBody {
    #[serde(default = "default_true")]
    enabled: bool,
    currency: Option<String>,
    override_name: Option<String>,
    store_message: Option<String>,
    /// Omitted keeps the current setting (`KEEP` for a new store).
    money_on_revoke: Option<MoneyOnRevoke>,
}

#[derive(Deserialize)]
struct CreateStoreBody {
    server_id: i64,
    #[serde(flatten)]
    store: StoreBody,
}

fn default_true() -> bool {
    true
}

/// Creates a server's store. 409 if it already has one.
#[post("/admin/store")]
async fn create_store(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<CreateStoreBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let body = body.into_inner();
    let details = normalize_store(body.store)?;
    allowed_currency(&details.currency)?;
    Server::find_by_id(&state.db, body.server_id)
        .await?
        .ok_or(ApiError::NotFound("server"))?;
    let store = ItemStore::create(&state.db, body.server_id, &details)
        .await?
        .ok_or(ApiError::Conflict("a store for this server"))?;
    log::info!("user {} created store {} for server {}", user.0.id, store.id, store.server_id);
    Ok(HttpResponse::Created().json(store))
}

#[derive(Serialize)]
struct AdminStorePage {
    #[serde(flatten)]
    store: StoreSummary,
    server: Server,
    /// Every discount, active or not.
    discounts: Vec<StoreDiscount>,
    /// Every item that isn't archived, available or not.
    items: Vec<PricedItem>,
}

/// A store in full, for its management page.
#[get("/admin/store/{store_id}")]
async fn get_store(state: web::Data<AppState>, user: AuthUser, store_id: web::Path<i64>) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (store, server) = find_store(&state, store_id.into_inner()).await?;
    let (discounts, items) = tokio::try_join!(
        StoreDiscount::list(&state.db, store.store.id),
        StoreItem::list(&state.db, store.store.id),
    )?;
    let items = items.into_iter().map(|i| PricedItem::new(i, store.store_discount)).collect();
    Ok(HttpResponse::Ok().json(AdminStorePage { store, server, discounts, items }))
}

/// Replaces a store's settings (not its grantable usergroups). The currency can only change
/// while the store has no items: their prices are in its minor units, so changing it would
/// re-price every item (1000 cents becoming ¥1000).
#[put("/admin/store/{store_id}")]
async fn update_store(
    state: web::Data<AppState>,
    user: AuthUser,
    store_id: web::Path<i64>,
    body: web::Json<StoreBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let details = normalize_store(body.into_inner())?;
    let (current, _) = find_store(&state, store_id.into_inner()).await?;
    // A store set up before the currency list may keep its currency, but not switch to
    // another one off the list.
    if current.store.currency != details.currency {
        if current.item_count > 0 {
            return Err(bad(format!(
                "the currency can't change while the store has items (their prices are in {}); archive them first",
                current.store.currency.to_ascii_uppercase()
            )));
        }
        allowed_currency(&details.currency)?;
    }
    let store = ItemStore::update(&state.db, current.store.id, &details)
        .await?
        .ok_or(ApiError::NotFound("store"))?;
    log::info!("user {} updated store {}", user.0.id, store.id);
    Ok(HttpResponse::Ok().json(store))
}

#[derive(Deserialize)]
struct GrantableBody {
    grantable_usergroups: Vec<String>,
}

/// Replaces the usergroups a store's items may grant. Requires Elevated Control as well.
/// Items that grant a usergroup no longer allowed are made unavailable.
#[put("/admin/store/{store_id}/grantable-usergroups")]
async fn set_grantable_usergroups(
    state: web::Data<AppState>,
    user: AuthUser,
    store_id: web::Path<i64>,
    body: web::Json<GrantableBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, GRANTABLE).await?;
    let groups = normalize_usergroups(body.into_inner().grantable_usergroups)?;
    let (store, disabled) = ItemStore::set_grantable_usergroups(&state.db, store_id.into_inner(), &groups)
        .await?
        .ok_or(ApiError::NotFound("store"))?;
    log::info!(
        "user {} set the grantable usergroups of store {} to {:?}; {disabled} item(s) made unavailable",
        user.0.id, store.id, groups
    );
    Ok(HttpResponse::Ok().json(store))
}

#[derive(Deserialize)]
struct ItemBody {
    name: String,
    #[serde(default)]
    short_description: String,
    /// Markdown; may be empty.
    #[serde(default)]
    full_description: String,
    image_url: Option<String>,
    #[serde(default)]
    sort_order: i32,
    #[serde(default = "default_true")]
    available: bool,
    /// Minor units.
    price: i64,
    #[serde(default)]
    discount: i32,
    starting_stock: Option<i32>,
    max_per_user: Option<i32>,
    #[serde(default)]
    rewards: Vec<Reward>,
}

/// Adds an item to a store.
#[post("/admin/store/{store_id}/items")]
async fn create_item(
    state: web::Data<AppState>,
    user: AuthUser,
    store_id: web::Path<i64>,
    body: web::Json<ItemBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (store, server) = find_store(&state, store_id.into_inner()).await?;
    let details = normalize_item(body.into_inner(), &store.store, &server)?;
    let item = StoreItem::create(&state.db, store.store.id, &details).await?;
    log::info!("user {} created item {} ({:?}) in store {}", user.0.id, item.id, item.name, store.store.id);
    Ok(HttpResponse::Created().json(PricedItem::new(item, store.store_discount)))
}

/// Replaces an item's details.
#[put("/admin/store/{store_id}/items/{item_id}")]
async fn update_item(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, i64)>,
    body: web::Json<ItemBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (store_id, item_id) = path.into_inner();
    let (store, server) = find_store(&state, store_id).await?;
    let details = normalize_item(body.into_inner(), &store.store, &server)?;
    let item = StoreItem::update(&state.db, store.store.id, item_id, &details)
        .await?
        .ok_or(ApiError::NotFound("item"))?;
    log::info!("user {} updated item {} in store {}", user.0.id, item.id, store.store.id);
    Ok(HttpResponse::Ok().json(PricedItem::new(item, store.store_discount)))
}

/// Archives an item: hidden from players, kept for receipts and fulfillments.
#[delete("/admin/store/{store_id}/items/{item_id}")]
async fn archive_item(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (store_id, item_id) = path.into_inner();
    if !StoreItem::archive(&state.db, store_id, item_id).await? {
        return Err(ApiError::NotFound("item"));
    }
    log::info!("user {} archived item {item_id} in store {store_id}", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
struct DiscountBody {
    discount: i32,
    #[serde(default)]
    message: String,
    starts_at: Option<DateTime<Utc>>,
    ends_at: Option<DateTime<Utc>>,
}

/// Adds a store-wide discount.
#[post("/admin/store/{store_id}/discounts")]
async fn create_discount(
    state: web::Data<AppState>,
    user: AuthUser,
    store_id: web::Path<i64>,
    body: web::Json<DiscountBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (store, _) = find_store(&state, store_id.into_inner()).await?;
    let details = normalize_discount(body.into_inner())?;
    let discount = StoreDiscount::create(&state.db, store.store.id, &details).await?;
    log::info!("user {} created discount {} ({}%) in store {}", user.0.id, discount.id, discount.discount, store.store.id);
    Ok(HttpResponse::Created().json(discount))
}

#[put("/admin/store/{store_id}/discounts/{discount_id}")]
async fn update_discount(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, i64)>,
    body: web::Json<DiscountBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (store_id, discount_id) = path.into_inner();
    let details = normalize_discount(body.into_inner())?;
    let discount = StoreDiscount::update(&state.db, store_id, discount_id, &details)
        .await?
        .ok_or(ApiError::NotFound("discount"))?;
    log::info!("user {} updated discount {} in store {store_id}", user.0.id, discount.id);
    Ok(HttpResponse::Ok().json(discount))
}

#[delete("/admin/store/{store_id}/discounts/{discount_id}")]
async fn delete_discount(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (store_id, discount_id) = path.into_inner();
    if !StoreDiscount::delete(&state.db, store_id, discount_id).await? {
        return Err(ApiError::NotFound("discount"));
    }
    log::info!("user {} deleted discount {discount_id} in store {store_id}", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

/// Query string of the receipt list. Blank values are ignored.
#[derive(Deserialize)]
struct ReceiptQuery {
    status: Option<String>,
    store: Option<String>,
    buyer: Option<String>,
    /// 1 to [`MAX_RECEIPTS`]; [`DEFAULT_RECEIPTS`] by default.
    limit: Option<String>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
}

/// Receipts matching the filters, newest first, a page at a time: `{ items, next_cursor }`.
/// Requires Admin Access and Store Finance.
#[get("/admin/store/receipts")]
async fn list_receipts(
    state: web::Data<AppState>,
    user: AuthUser,
    query: web::Query<ReceiptQuery>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, FINANCE).await?;
    let filter = parse_receipt_filter(query.into_inner())?;
    let receipts: Vec<ReceiptView> = Receipt::list(&state.db, &filter).await?;
    let page = Page::new(receipts, filter.limit, |r| Cursor { created_at: r.receipt.purchased_at, id: r.receipt.id });
    Ok(HttpResponse::Ok().json(page))
}

/// Moves a receipt back to paid and has the server's addon deliver its rewards again (see
/// `Receipt::reinstate`): a disputed one whose dispute was won, or a refunded one whose refund
/// failed, when staff would rather the buyer kept the purchase than try refunding again.
/// Requires Admin Access and Store Finance. 400 for any other receipt.
#[post("/admin/store/receipts/{receipt_id}/reinstate")]
async fn reinstate(
    state: web::Data<AppState>,
    user: AuthUser,
    receipt_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, FINANCE).await?;
    let id = receipt_id.into_inner();
    let view = Receipt::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("receipt"))?;
    let receipt = view.receipt;
    let after_dispute = match receipt.status {
        ReceiptStatus::Disputed => {
            if !receipt.dispute_outcome.as_deref().is_some_and(|o| DISPUTE_WON.contains(&o)) {
                return Err(bad(match receipt.dispute_outcome.as_deref() {
                    None => "its dispute hasn't closed yet; it can be reinstated once the dispute is won".to_owned(),
                    Some(outcome) => format!("its dispute wasn't won ({outcome}), so the payment wasn't kept"),
                }));
            }
            true
        }
        ReceiptStatus::Refunded if view.refunds.as_ref().is_some_and(|r| r.falls_short()) => false,
        ReceiptStatus::Refunded => {
            return Err(bad("it was refunded; only a refund that failed leaves a purchase to reinstate".to_owned()));
        }
        ReceiptStatus::Paid => return Err(bad("it's paid already".to_owned())),
    };
    let (reinstated, fulfillment) = Receipt::reinstate(&state.db, id)
        .await?
        .ok_or_else(|| bad("this receipt has changed meanwhile; reload and try again".to_owned()))?;
    log::info!(
        "user {} reinstated {} receipt {} for user {}; fulfillment {} is {}",
        user.0.id,
        if after_dispute { "disputed" } else { "refunded (refund failed)" },
        reinstated.id,
        reinstated.buyer_id,
        fulfillment.as_ref().map_or("none".to_owned(), |f| f.id.to_string()),
        fulfillment.as_ref().map_or("missing", |f| match f.status {
            FulfillmentStatus::Fulfilled => "delivered already",
            _ => "to be delivered again",
        }),
    );
    notify::purchase_reinstated(&state.db, &reinstated, user.0.id, after_dispute).await;
    let updated = Receipt::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("receipt"))?;
    Ok(HttpResponse::Ok().json(updated))
}

/// Refunds a receipt in full through Stripe and revokes its fulfillment. Requires Admin
/// Access and Store Finance. 400 unless the receipt is paid, or refunded with a refund that
/// failed (refunding it again refunds whatever Stripe hasn't).
#[post("/admin/store/receipts/{receipt_id}/refund")]
async fn refund(
    state: web::Data<AppState>,
    user: AuthUser,
    receipt_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, FINANCE).await?;
    let stripe = state.stripe()?;
    let view = Receipt::find_by_id(&state.db, receipt_id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("receipt"))?;
    let receipt = &view.receipt;
    let failed_refund = receipt.status == ReceiptStatus::Refunded && view.refunds.as_ref().is_some_and(|r| r.falls_short());
    if receipt.status != ReceiptStatus::Paid && !failed_refund {
        return Err(bad(format!("receipt is already {}", format!("{:?}", receipt.status).to_lowercase())));
    }
    refund_receipt(&state, stripe, &view, user.0.id).await?;
    let updated = Receipt::find_by_id(&state.db, receipt.id)
        .await?
        .ok_or(ApiError::NotFound("receipt"))?;
    Ok(HttpResponse::Ok().json(updated))
}

/// Refunds a receipt in full (whatever Stripe hasn't refunded yet) through Stripe, marks it
/// refunded (revoking its fulfillment) and tells the buyer; `by` is the staff member. 400
/// for a receipt recorded without a payment, 422 if Stripe refuses or fails the refund at
/// once, leaving the receipt as it was. Stripe refunds are idempotent per receipt until one
/// fails (see `PaymentRefunds::refund_key`), so refunding one twice, even at the same moment,
/// refunds it once.
///
/// Most refunds succeed at once, but some stay pending and a few fail later (a closed card,
/// say). The receipt is marked refunded as soon as Stripe accepts the refund, then what
/// Stripe has refunded is recorded, now and on each of its refund events: a refund that
/// fails later flags the receipt for staff to refund again or reinstate (see
/// `routes::store::sync_refunds`).
async fn refund_receipt(state: &AppState, stripe: &Stripe, view: &ReceiptView, by: i64) -> Result<(), ApiError> {
    let receipt = &view.receipt;
    // Sessions that needed no payment are recorded with a placeholder instead of a
    // PaymentIntent; there's nothing to refund through Stripe.
    if receipt.stripe_payment_intent_id.starts_with("session:") {
        return Err(bad("this receipt has no payment to refund".to_owned()));
    }
    let payment_intent = receipt.stripe_payment_intent_id.as_str();
    let key = PaymentRefunds::refund_key(view.refunds.as_deref(), receipt.id);
    // `None` if it was already refunded in full (e.g. from the Stripe dashboard, with the
    // webhook not yet arrived); the receipt is still marked refunded below.
    let stripe_refund = stripe.refund(payment_intent, &key).await?;
    if let Some(failed) = stripe_refund.as_ref().filter(|r| r.failed()) {
        // Recorded so that trying again uses a new idempotency key.
        if let Err(e) = sync_refunds(state, Some(payment_intent), "failed refund").await {
            log::warn!("couldn't record the refunds of receipt {}: {e}", receipt.id);
        }
        log::warn!(
            "user {by} tried to refund receipt {}, but Stripe's refund {} {}: {}",
            receipt.id,
            failed.id,
            failed.status.as_deref().unwrap_or("failed"),
            failed.failure_reason.as_deref().unwrap_or("no reason given")
        );
        return Err(ApiError::StripeRejected(format!(
            "Stripe couldn't refund this payment ({})",
            failed.failure_reason.as_deref().unwrap_or("the refund failed")
        )));
    }
    // The `charge.refunded` webhook does the same later; it finds nothing left to change
    // (or, if it got here first, we don't), so the buyer is notified once.
    if let Some((refunded, _)) = Receipt::set_status(&state.db, receipt.id, ReceiptStatus::Refunded).await? {
        notify::purchase_refunded(&state.db, &refunded, Some(by)).await;
    }
    log::info!(
        "user {by} refunded receipt {} ({} {}) for user {}: stripe refund {} ({})",
        receipt.id, receipt.price, receipt.currency, receipt.buyer_id,
        stripe_refund.as_ref().map_or("none", |r| r.id.as_str()),
        stripe_refund.as_ref().and_then(|r| r.status.as_deref()).unwrap_or("already refunded")
    );
    // Shows the refund as pending (or done) on the receipt right away; Stripe's webhook would
    // record it shortly anyway.
    if let Err(e) = sync_refunds(state, Some(payment_intent), "refund by staff").await {
        log::warn!("couldn't record the refunds of receipt {} yet: {e}", receipt.id);
    }
    Ok(())
}

/// Paid purchases from a server's store that haven't been delivered yet.
#[derive(Serialize)]
struct Undelivered {
    count: i64,
    /// The server's store, if it has one (for linking to its receipts).
    store_id: Option<i64>,
}

/// How many paid purchases from a server's store haven't been delivered, which keeps the
/// server from being deleted. Requires Admin Access and Server Management or Store Finance.
#[get("/admin/store/servers/{server_id}/undelivered")]
async fn undelivered(state: web::Data<AppState>, user: AuthUser, server_id: web::Path<i64>) -> Result<HttpResponse, ApiError> {
    let permissions = user.require(&state.db, Permissions::ADMIN_ACCESS).await?;
    if !permissions.contains(Permissions::SERVER_MANAGEMENT) && !permissions.contains(Permissions::STORE_FINANCE) {
        return Err(ApiError::Forbidden(format!(
            "{} or {}",
            Permissions::SERVER_MANAGEMENT.name(),
            Permissions::STORE_FINANCE.name()
        )));
    }
    let server_id = server_id.into_inner();
    Server::find_by_id(&state.db, server_id)
        .await?
        .ok_or(ApiError::NotFound("server"))?;
    let count = Receipt::count_undelivered(&mut *state.db.acquire().await?, server_id).await?;
    let store = ItemStore::find_by_server(&state.db, server_id).await?;
    Ok(HttpResponse::Ok().json(Undelivered { count, store_id: store.map(|s| s.id) }))
}

/// What a mass refund did.
#[derive(Serialize)]
struct MassRefund {
    /// Receipts refunded by this request.
    refunded: usize,
    /// Receipts that couldn't be refunded, and why.
    failed: Vec<RefundFailure>,
    /// Paid purchases still undelivered afterwards: ones that failed, and any left for
    /// another request.
    remaining: i64,
}

#[derive(Serialize)]
struct RefundFailure {
    receipt_id: i64,
    error: String,
}

/// At most this many receipts are refunded per request...
const MASS_REFUND_MAX: i64 = 100;
/// ...and no refund is started after this long, so the request stays well within the
/// frontend's timeout. Each refund is a request to Stripe; the rest wait for the next run.
const MASS_REFUND_BUDGET: Duration = Duration::from_secs(20);

/// Refunds every paid purchase from a server's store that hasn't been delivered, oldest
/// first, as `POST /admin/store/receipts/{id}/refund` would one by one: in full through
/// Stripe, revoking each fulfillment and telling each buyer. For settling a server's
/// purchases before it's deleted. Requires Admin Access and Store Finance.
///
/// A receipt Stripe refuses is reported and skipped; a Stripe outage stops the run. Large
/// stores take several runs (see [`MASS_REFUND_BUDGET`]); `remaining` says how many are left.
#[post("/admin/store/servers/{server_id}/undelivered/refund")]
async fn refund_undelivered(
    state: web::Data<AppState>,
    user: AuthUser,
    server_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, FINANCE).await?;
    let stripe = state.stripe()?;
    let server_id = server_id.into_inner();
    Server::find_by_id(&state.db, server_id)
        .await?
        .ok_or(ApiError::NotFound("server"))?;

    let started = Instant::now();
    let mut refunded = 0;
    let mut failed = Vec::new();
    for receipt in Receipt::undelivered(&state.db, server_id, MASS_REFUND_MAX).await? {
        if started.elapsed() >= MASS_REFUND_BUDGET {
            break;
        }
        // Delivered (or refunded) since the list was read: leave it be.
        if !Receipt::is_undelivered(&state.db, server_id, receipt.id).await? {
            continue;
        }
        let Some(view) = Receipt::find_by_id(&state.db, receipt.id).await? else {
            continue;
        };
        let error = match refund_receipt(&state, stripe, &view, user.0.id).await {
            Ok(()) => {
                refunded += 1;
                continue;
            }
            // Stripe's own message, or ours; both are safe to show.
            Err(ApiError::StripeRejected(message) | ApiError::BadRequest(message)) => message,
            Err(ApiError::StripeUnavailable(e)) => {
                log::warn!("mass refund of server {server_id} stopped at receipt {}: {e}", receipt.id);
                failed.push(RefundFailure {
                    receipt_id: receipt.id,
                    error: "the payment provider couldn't be reached; try again later".to_owned(),
                });
                break;
            }
            Err(e) => return Err(e),
        };
        log::warn!("mass refund of server {server_id}: receipt {} not refunded: {error}", receipt.id);
        failed.push(RefundFailure { receipt_id: receipt.id, error });
    }

    let remaining = Receipt::count_undelivered(&mut *state.db.acquire().await?, server_id).await?;
    log::info!(
        "user {} refunded {refunded} undelivered purchase(s) of server {server_id}; {} failed, {remaining} still undelivered",
        user.0.id,
        failed.len()
    );
    Ok(HttpResponse::Ok().json(MassRefund { refunded, failed, remaining }))
}

/// Offers a failed fulfillment to the server's addon again with a fresh set of attempts,
/// once whatever made it fail is fixed. Requires Admin Access and Store Finance. 400 unless
/// it's failed and its receipt is still paid.
#[post("/admin/store/fulfillments/{fulfillment_id}/retry")]
async fn retry_fulfillment(
    state: web::Data<AppState>,
    user: AuthUser,
    fulfillment_id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, FINANCE).await?;
    let id = fulfillment_id.into_inner();
    let fulfillment = Fulfillment::retry(&state.db, id)
        .await?
        .ok_or_else(|| bad("only a failed delivery of a paid purchase can be retried".to_owned()))?;
    log::info!("user {} reset fulfillment {id} for another round of delivery attempts", user.0.id);
    let receipt = Receipt::find_by_id(&state.db, fulfillment.receipt_id)
        .await?
        .ok_or(ApiError::NotFound("receipt"))?;
    Ok(HttpResponse::Ok().json(receipt))
}

/// A store (enabled or not) and its server.
async fn find_store(state: &AppState, id: i64) -> Result<(StoreSummary, Server), ApiError> {
    let store = ItemStore::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("store"))?;
    let server = Server::find_by_id(&state.db, store.store.server_id)
        .await?
        .ok_or(ApiError::NotFound("store"))?;
    Ok((store, server))
}

fn normalize_store(body: StoreBody) -> Result<StoreDetails, ApiError> {
    let currency = body.currency.as_deref().map(str::trim).filter(|c| !c.is_empty()).unwrap_or("usd");
    if currency.len() != 3 || !currency.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err(bad(format!("currency must be a three-letter ISO 4217 code, got {currency:?}")));
    }
    let override_name = text("override_name", body.override_name.as_deref().unwrap_or(""), MAX_NAME)?;
    let store_message = text("store_message", body.store_message.as_deref().unwrap_or(""), MAX_MESSAGE)?;
    Ok(StoreDetails {
        enabled: body.enabled,
        currency: currency.to_ascii_lowercase(),
        override_name: Some(override_name).filter(|n| !n.is_empty()),
        store_message: Some(store_message).filter(|m| !m.is_empty()),
        money_on_revoke: body.money_on_revoke,
    })
}

/// 400 unless `currency` (already lowercased) is one stores may use.
fn allowed_currency(currency: &str) -> Result<(), ApiError> {
    if CURRENCIES.contains(&currency) {
        return Ok(());
    }
    let list: Vec<String> = CURRENCIES.iter().map(|c| c.to_ascii_uppercase()).collect();
    Err(bad(format!(
        "{} isn't a supported currency; use one of {}",
        currency.to_ascii_uppercase(),
        list.join(", ")
    )))
}

/// Trimmed, deduplicated usergroup names.
fn normalize_usergroups(groups: Vec<String>) -> Result<Vec<String>, ApiError> {
    let mut out: Vec<String> = Vec::new();
    for group in groups {
        let group = required_text("grantable_usergroups", &group, MAX_USERGROUP)?;
        if group.chars().any(char::is_whitespace) {
            return Err(bad(format!("usergroup names can't contain spaces, got {group:?}")));
        }
        if !out.contains(&group) {
            out.push(group);
        }
    }
    if out.len() > MAX_USERGROUPS {
        return Err(bad(format!("at most {MAX_USERGROUPS} grantable usergroups")));
    }
    Ok(out)
}

fn normalize_item(body: ItemBody, store: &ItemStore, server: &Server) -> Result<ItemDetails, ApiError> {
    if !(0..=MAX_PRICE).contains(&body.price) {
        return Err(bad(format!("price must be between 0 and {MAX_PRICE} (in minor units, e.g. cents)")));
    }
    if !(0..=100).contains(&body.discount) {
        return Err(bad("discount must be between 0 and 100".to_owned()));
    }
    if body.starting_stock.is_some_and(|s| !(0..=MAX_STOCK).contains(&s)) {
        return Err(bad(format!("starting_stock must be between 0 and {MAX_STOCK}")));
    }
    if body.max_per_user.is_some_and(|m| !(1..=MAX_PER_USER).contains(&m)) {
        return Err(bad(format!("max_per_user must be between 1 and {MAX_PER_USER}")));
    }
    if body.rewards.is_empty() {
        return Err(bad("an item needs at least one reward".to_owned()));
    }
    if body.rewards.len() > MAX_REWARDS {
        return Err(bad(format!("at most {MAX_REWARDS} rewards per item")));
    }
    let rewards = body
        .rewards
        .into_iter()
        .map(|r| normalize_reward(r, store, server))
        .collect::<Result<Vec<_>, _>>()?;
    let full_description = if body.full_description.trim().is_empty() {
        String::new()
    } else {
        markdown("full_description", &body.full_description, MAX_FULL_DESCRIPTION)?
    };
    Ok(ItemDetails {
        name: required_text("name", &body.name, MAX_NAME)?,
        short_description: text("short_description", &body.short_description, MAX_SHORT_DESCRIPTION)?,
        full_description,
        image_url: optional_url("image_url", body.image_url.as_deref())?,
        sort_order: body.sort_order,
        available: body.available,
        price: body.price,
        discount: body.discount,
        starting_stock: body.starting_stock,
        max_per_user: body.max_per_user,
        rewards,
    })
}

/// Checks a reward against the store (grantable usergroups) and server (gamemode).
fn normalize_reward(reward: Reward, store: &ItemStore, server: &Server) -> Result<Reward, ApiError> {
    match reward {
        Reward::Usergroup { usergroup, duration_seconds } => {
            let usergroup = required_text("usergroup", &usergroup, MAX_USERGROUP)?;
            if !store.grantable_usergroups.contains(&usergroup) {
                return Err(bad(format!(
                    "usergroup {usergroup:?} isn't one this store may grant; ask a user with Elevated Control to allow it"
                )));
            }
            if duration_seconds.is_some_and(|d| !(1..=MAX_DURATION_SECONDS).contains(&d)) {
                return Err(bad(format!("duration_seconds must be between 1 and {MAX_DURATION_SECONDS}")));
            }
            Ok(Reward::Usergroup { usergroup, duration_seconds })
        }
        Reward::DarkrpMoney { amount } => {
            // An unlinked server hasn't reported its gamemode yet; trust the manager then.
            if !server.gamemode.is_empty() && !server.gamemode.to_ascii_lowercase().contains("darkrp") {
                return Err(bad(format!(
                    "darkrp_money rewards need a DarkRP server; {} runs {:?}",
                    server.name, server.gamemode
                )));
            }
            if !(1..=MAX_MONEY).contains(&amount) {
                return Err(bad(format!("darkrp_money amount must be between 1 and {MAX_MONEY}")));
            }
            Ok(Reward::DarkrpMoney { amount })
        }
        Reward::Weapon { class } => {
            let class = required_text("weapon class", &class, MAX_WEAPON_CLASS)?;
            if class.chars().any(char::is_whitespace) {
                return Err(bad(format!("weapon class can't contain spaces, got {class:?}")));
            }
            Ok(Reward::Weapon { class })
        }
    }
}

fn normalize_discount(body: DiscountBody) -> Result<DiscountDetails, ApiError> {
    if !(0..=100).contains(&body.discount) {
        return Err(bad("discount must be between 0 and 100".to_owned()));
    }
    if let (Some(starts), Some(ends)) = (body.starts_at, body.ends_at)
        && ends <= starts
    {
        return Err(bad("ends_at must be after starts_at".to_owned()));
    }
    Ok(DiscountDetails {
        discount: body.discount,
        message: text("message", &body.message, MAX_MESSAGE)?,
        starts_at: body.starts_at,
        ends_at: body.ends_at,
    })
}

fn parse_receipt_filter(query: ReceiptQuery) -> Result<ReceiptFilter, ApiError> {
    fn present(value: Option<String>) -> Option<String> {
        value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
    }
    fn id(field: &str, value: Option<String>) -> Result<Option<i64>, ApiError> {
        present(value)
            .map(|v| v.parse::<i64>().map_err(|_| bad(format!("{field} must be a number"))))
            .transpose()
    }
    let status = present(query.status)
        .map(|v| {
            serde_json::from_value::<ReceiptStatus>(serde_json::Value::String(v.to_ascii_uppercase()))
                .map_err(|_| bad(format!("status must be one of paid, refunded, disputed, got {v:?}")))
        })
        .transpose()?;
    let limit = id("limit", query.limit)?.unwrap_or(DEFAULT_RECEIPTS);
    if !(1..=MAX_RECEIPTS).contains(&limit) {
        return Err(bad(format!("limit must be between 1 and {MAX_RECEIPTS}")));
    }
    let after = present(query.cursor)
        .map(|v| Cursor::parse(&v).ok_or_else(|| bad("cursor is not a next_cursor value".to_owned())))
        .transpose()?;
    Ok(ReceiptFilter {
        status,
        store_id: id("store", query.store)?,
        buyer_id: id("buyer", query.buyer)?,
        after,
        limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> ItemStore {
        ItemStore {
            id: 1,
            server_id: 1,
            enabled: true,
            currency: "usd".into(),
            override_name: None,
            store_message: None,
            grantable_usergroups: vec!["vip".into()],
            money_on_revoke: MoneyOnRevoke::Keep,
            created_at: Utc::now(),
            last_updated: None,
        }
    }

    fn server(gamemode: &str) -> Server {
        Server {
            id: 1,
            name: "Test".into(),
            description: String::new(),
            gamemode: gamemode.into(),
            splash_url: None,
            icon_url: None,
            color: "#3b82f6".into(),
            connection_address: String::new(),
            map: String::new(),
            last_heartbeat: None,
            linked: true,
            created_at: Utc::now(),
        }
    }

    fn item() -> ItemBody {
        ItemBody {
            name: " VIP ".into(),
            short_description: "A month of VIP".into(),
            full_description: String::new(),
            image_url: Some(String::new()),
            sort_order: 0,
            available: true,
            price: 499,
            discount: 10,
            starting_stock: None,
            max_per_user: Some(1),
            rewards: vec![Reward::Usergroup { usergroup: "vip".into(), duration_seconds: Some(30 * 86_400) }],
        }
    }

    #[test]
    fn stores() {
        let d = normalize_store(StoreBody {
            enabled: true,
            currency: Some(" EUR ".into()),
            override_name: Some("  ".into()),
            store_message: Some("Hi".into()),
            money_on_revoke: Some(MoneyOnRevoke::Deduct),
        })
        .unwrap_or_else(|_| panic!("valid store rejected"));
        assert_eq!(d.currency, "eur");
        assert_eq!(d.money_on_revoke, Some(MoneyOnRevoke::Deduct));
        assert_eq!(d.override_name, None);
        assert_eq!(d.store_message.as_deref(), Some("Hi"));
        assert!(normalize_store(StoreBody { enabled: true, currency: Some("dollars".into()), override_name: None, store_message: None, money_on_revoke: None }).is_err());
        assert_eq!(normalize_store(StoreBody { enabled: false, currency: None, override_name: None, store_message: None, money_on_revoke: None }).unwrap().currency, "usd");

        assert!(allowed_currency("usd").is_ok());
        assert!(allowed_currency("jpy").is_ok());
        assert!(allowed_currency("xyz").is_err());
        assert!(allowed_currency("USD").is_err(), "callers lowercase first");
    }

    #[test]
    fn items() {
        let d = normalize_item(item(), &store(), &server("DarkRP")).unwrap_or_else(|_| panic!("valid item rejected"));
        assert_eq!(d.name, "VIP");
        assert_eq!(d.image_url, None);
        assert_eq!(d.rewards.len(), 1);

        assert!(normalize_item(ItemBody { price: -1, ..item() }, &store(), &server("")).is_err());
        assert!(normalize_item(ItemBody { discount: 101, ..item() }, &store(), &server("")).is_err());
        assert!(normalize_item(ItemBody { rewards: vec![], ..item() }, &store(), &server("")).is_err());
        assert!(normalize_item(ItemBody { max_per_user: Some(0), ..item() }, &store(), &server("")).is_err());
        assert!(normalize_item(ItemBody { full_description: "[x](javascript:1)".into(), ..item() }, &store(), &server("")).is_err());
    }

    #[test]
    fn rewards() {
        let s = store();
        let vip = Reward::Usergroup { usergroup: "vip".into(), duration_seconds: None };
        assert!(normalize_reward(vip, &s, &server("")).is_ok());
        let admin = Reward::Usergroup { usergroup: "superadmin".into(), duration_seconds: None };
        assert!(normalize_reward(admin, &s, &server("")).is_err());
        let money = Reward::DarkrpMoney { amount: 1000 };
        assert!(normalize_reward(money.clone(), &s, &server("darkrp")).is_ok());
        assert!(normalize_reward(money.clone(), &s, &server("")).is_ok());
        assert!(normalize_reward(money, &s, &server("Sandbox")).is_err());
        assert!(normalize_reward(Reward::DarkrpMoney { amount: 0 }, &s, &server("darkrp")).is_err());
        assert!(normalize_reward(Reward::Weapon { class: "weapon_crowbar".into() }, &s, &server("")).is_ok());
        assert!(normalize_reward(Reward::Weapon { class: "weapon crowbar".into() }, &s, &server("")).is_err());
    }

    #[test]
    fn discounts_and_filters() {
        let now = Utc::now();
        assert!(normalize_discount(DiscountBody { discount: 25, message: String::new(), starts_at: Some(now), ends_at: Some(now) }).is_err());
        assert!(normalize_discount(DiscountBody { discount: 25, message: String::new(), starts_at: None, ends_at: Some(now) }).is_ok());
        assert!(normalize_discount(DiscountBody { discount: 200, message: String::new(), starts_at: None, ends_at: None }).is_err());

        let f = parse_receipt_filter(ReceiptQuery { status: Some("refunded".into()), store: Some("3".into()), buyer: Some("".into()), limit: None, cursor: None }).unwrap();
        assert_eq!(f.status, Some(ReceiptStatus::Refunded));
        assert_eq!(f.store_id, Some(3));
        assert_eq!(f.buyer_id, None);
        assert!(parse_receipt_filter(ReceiptQuery { status: Some("pending".into()), store: None, buyer: None, limit: None, cursor: None }).is_err());
        assert!(parse_receipt_filter(ReceiptQuery { status: None, store: Some("x".into()), buyer: None, limit: None, cursor: None }).is_err());
        let page = |limit: &str, cursor: &str| {
            parse_receipt_filter(ReceiptQuery {
                status: None,
                store: None,
                buyer: None,
                limit: Some(limit.into()),
                cursor: Some(cursor.into()),
            })
        };
        let f = page("", "").unwrap();
        assert_eq!((f.limit, f.after), (DEFAULT_RECEIPTS, None));
        let f = page("50", "1794000000123456-42").unwrap();
        assert_eq!((f.limit, f.after), (50, Cursor::parse("1794000000123456-42")));
        assert!(page("0", "").is_err());
        assert!(page(&(MAX_RECEIPTS + 1).to_string(), "").is_err());
        assert!(page("", "nope").is_err());
        assert!(normalize_usergroups(vec!["vip".into(), " vip".into(), "donor".into()]).unwrap().len() == 2);
        assert!(normalize_usergroups(vec!["super admin".into()]).is_err());
    }
}
