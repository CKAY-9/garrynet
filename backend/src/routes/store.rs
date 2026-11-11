//! The store as players see it (docs/STORE.md): browsing is public, buying needs a logged-in
//! user with Site Access and goes through a Stripe Checkout Session. Stripe reports back
//! through the webhook, which turns a paid session into a receipt and a fulfillment.
//! Managing stores lives in `routes::store_admin`.

use actix_web::{HttpRequest, HttpResponse, get, post, web};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Server, Usergroup,
        store::{
            Begin, CHECKOUT_TTL_MINUTES, ItemStore, MAX_PENDING_PER_BUYER, MarkPaid, PricedItem, Purchase, PurchaseStatus,
            Receipt, ReceiptStatus, ReceiptView, StoreDiscount, StoreItem, StoreSummary,
            ungrantable_usergroup,
        },
    },
    notify,
    state::AppState,
    stripe::{CheckoutParams, Event, decode_hex},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    // `purchases` before `{store_id}`, so it isn't matched as a store id.
    cfg.service(list_stores)
        .service(my_purchases)
        .service(cancel_purchase)
        .service(get_store)
        .service(get_item)
        .service(checkout)
        .service(webhook);
}

/// Without Site Access a user is banned: they can browse but not buy.
const BUY: Permissions = Permissions::SITE_ACCESS;

/// A store with the server it belongs to.
#[derive(Serialize)]
pub struct StoreWithServer {
    #[serde(flatten)]
    pub store: StoreSummary,
    pub server: Server,
}

/// Every enabled store. Public.
#[get("/store")]
async fn list_stores(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let (stores, servers) = tokio::try_join!(ItemStore::list_enabled(&state.db), Server::list(&state.db))?;
    let stores = stores.into_iter().map(public).collect();
    Ok(HttpResponse::Ok().json(with_servers(stores, servers)))
}

/// A store as the public sees it: which usergroups items may grant is for managers only.
fn public(mut store: StoreSummary) -> StoreSummary {
    store.store.grantable_usergroups.clear();
    store
}

/// Pairs each store with its server. Stores whose server is gone (can't happen: the store
/// cascades) are skipped rather than failing the page.
pub fn with_servers(stores: Vec<StoreSummary>, servers: Vec<Server>) -> Vec<StoreWithServer> {
    stores
        .into_iter()
        .filter_map(|store| {
            let server = servers.iter().find(|s| s.id == store.store.server_id)?.clone();
            Some(StoreWithServer { store, server })
        })
        .collect()
}

#[derive(Serialize)]
struct StorePage {
    #[serde(flatten)]
    store: StoreSummary,
    server: Server,
    /// Store-wide discounts in force.
    discounts: Vec<StoreDiscount>,
    /// Items that aren't archived, in display order, priced after every discount.
    items: Vec<PricedItem>,
}

/// One enabled store with its items. Public.
#[get("/store/{store_id}")]
async fn get_store(state: web::Data<AppState>, store_id: web::Path<i64>) -> Result<HttpResponse, ApiError> {
    let (store, server) = find_enabled_store(&state, store_id.into_inner()).await?;
    let (discounts, items) = tokio::try_join!(
        StoreDiscount::list_active(&state.db, store.store.id),
        StoreItem::list(&state.db, store.store.id),
    )?;
    let items = items.into_iter().map(|i| PricedItem::new(i, store.store_discount)).collect();
    Ok(HttpResponse::Ok().json(StorePage { store: public(store), server, discounts, items }))
}

/// Whether the requester may buy an item, and why not.
#[derive(Serialize)]
struct Eligibility {
    /// How many times they've bought it (paid or pending).
    purchased: i64,
    can_buy: bool,
    /// Human-readable, when `can_buy` is false.
    reason: Option<String>,
}

#[derive(Serialize)]
struct ItemPage {
    store: StoreSummary,
    server: Server,
    item: PricedItem,
    /// Present when logged in.
    buyer: Option<Eligibility>,
}

/// One item of an enabled store. Public; logged-in users also learn whether they can buy it.
#[get("/store/{store_id}/items/{item_id}")]
async fn get_item(
    state: web::Data<AppState>,
    path: web::Path<(i64, i64)>,
    user: Option<AuthUser>,
) -> Result<HttpResponse, ApiError> {
    let (store_id, item_id) = path.into_inner();
    let (store, server) = find_enabled_store(&state, store_id).await?;
    let item = StoreItem::find(&state.db, store.store.id, item_id)
        .await?
        .ok_or(ApiError::NotFound("item"))?;
    let item = PricedItem::new(item, store.store_discount);
    let buyer = match user {
        Some(user) => Some(eligibility(&state, &user, &store, &item).await?),
        None => None,
    };
    Ok(HttpResponse::Ok().json(ItemPage { item, store: public(store), server, buyer }))
}

/// Mirrors the checks of `Purchase::begin`, so the page doesn't offer a purchase that
/// checkout would refuse.
async fn eligibility(
    state: &AppState,
    user: &AuthUser,
    store: &StoreSummary,
    priced: &PricedItem,
) -> Result<Eligibility, ApiError> {
    let item = &priced.item;
    let (permissions, purchased) = tokio::try_join!(
        Usergroup::permissions_for_user(&state.db, user.0.id),
        Purchase::count_for_buyer(&state.db, user.0.id, item.id),
    )?;
    let reason = if !permissions.contains(BUY) {
        Some("your account can't make purchases".to_owned())
    } else if !item.available {
        Some("this item isn't available right now".to_owned())
    } else if ungrantable_usergroup(&item.rewards, &store.store.grantable_usergroups).is_some() {
        Some(NOT_GRANTABLE.to_owned())
    } else if priced.final_price <= 0 {
        Some(FREE_ITEM.to_owned())
    } else if item.remaining_stock == Some(0) {
        Some("this item is sold out".to_owned())
    } else if item.max_per_user.is_some_and(|max| purchased >= i64::from(max)) {
        Some("you've already bought this item".to_owned())
    } else if state.stripe.is_none() {
        Some("purchases aren't set up on this GarryNet yet".to_owned())
    } else {
        None
    };
    Ok(Eligibility { purchased, can_buy: reason.is_none(), reason })
}

#[derive(Serialize)]
struct Checkout {
    purchase_id: i64,
    /// The Stripe Checkout page to send the buyer to.
    url: String,
}

/// Starts a purchase: reserves stock, opens a Stripe Checkout Session and returns its URL.
/// The purchase stays pending until Stripe's webhook reports it paid or expired.
#[post("/store/{store_id}/items/{item_id}/checkout")]
async fn checkout(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    let stripe = state.stripe()?;
    user.require(&state.db, BUY).await?;
    let (store_id, item_id) = path.into_inner();
    let (store, _) = find_enabled_store(&state, store_id).await?;
    // Only checks that the item belongs to the store; `begin` reads it again under a lock.
    StoreItem::find(&state.db, store.store.id, item_id)
        .await?
        .ok_or(ApiError::NotFound("item"))?;

    // Every checkout opens a Stripe session, so starting them is rate limited.
    let window = CHECKOUT_WINDOW_SECS as f64;
    let (recent, oldest) = Purchase::recent_by_buyer(&state.db, user.0.id, window).await?;
    if recent >= CHECKOUT_BURST {
        let elapsed = oldest.map_or(0.0, |t| (Utc::now() - t).num_milliseconds() as f64 / 1000.0);
        return Err(ApiError::RateLimited((window - elapsed).ceil().max(1.0) as u64));
    }

    // A buyer who comes back for an item they didn't finish paying for starts over: the
    // old checkout is closed first, so it doesn't keep holding a unit of stock.
    for pending in Purchase::pending_for_buyer(&state.db, user.0.id, item_id).await? {
        if release_pending(&state, &pending).await?.is_none() {
            return Err(ApiError::BadRequest(
                "your last checkout of this item has been paid or its payment is still clearing; see My purchases".to_owned(),
            ));
        }
    }

    // The price and currency charged are the ones `begin` recorded on the purchase, read
    // under its locks, so an edit made meanwhile can't make the two disagree.
    let (purchase, item) = match Purchase::begin(&state.db, item_id, user.0.id, &user.0.steam_id).await? {
        Begin::Started { purchase, item } => (purchase, item),
        Begin::Unavailable => return Err(ApiError::Locked("item")),
        Begin::SoldOut => return Err(ApiError::BadRequest("this item is sold out".to_owned())),
        Begin::LimitReached => {
            return Err(ApiError::BadRequest("you've already bought this item as many times as allowed".to_owned()));
        }
        Begin::Free => return Err(ApiError::BadRequest(FREE_ITEM.to_owned())),
        Begin::NotGrantable(group) => {
            log::warn!("item {item_id} grants usergroup {group:?}, which its store no longer allows");
            return Err(ApiError::BadRequest(NOT_GRANTABLE.to_owned()));
        }
        // Another checkout of the item was started at the same moment.
        Begin::AlreadyPending => {
            return Err(ApiError::Conflict("an open checkout of this item"));
        }
        Begin::TooManyPending => {
            return Err(ApiError::BadRequest(format!(
                "you have {MAX_PENDING_PER_BUYER} checkouts open; finish or cancel one first"
            )));
        }
    };

    let frontend = &state.config.frontend_url;
    let params = CheckoutParams {
        purchase_id: purchase.id,
        buyer_id: user.0.id,
        item_id: item.id,
        currency: &purchase.currency,
        unit_amount: purchase.price,
        name: &item.name,
        description: &item.short_description,
        image_url: item.image_url.as_deref(),
        success_url: &format!("{frontend}/store/purchases?paid=1"),
        cancel_url: &format!(
            "{frontend}/store/checkout/cancel?purchase={}&token={}",
            purchase.id,
            cancel_token(&state.config.jwt_secret, purchase.id)
        ),
        expires_at: (Utc::now() + Duration::minutes(CHECKOUT_TTL_MINUTES)).timestamp(),
    };
    let session = match stripe.create_checkout_session(&params).await {
        Ok(session) => session,
        Err(e) => {
            // Give the stock back; the buyer never got a payment page.
            abandon_checkout(&state, purchase.id, None).await;
            return Err(e);
        }
    };
    let Some(url) = session.url.clone() else {
        abandon_checkout(&state, purchase.id, Some(&session.id)).await;
        return Err(ApiError::StripeUnavailable("checkout session has no URL".into()));
    };
    match Purchase::attach_session(&state.db, purchase.id, &session.id).await {
        Ok(Some(_)) => {}
        // Cancelled while the session was being created: don't hand out a payable session.
        Ok(None) => {
            abandon_checkout(&state, purchase.id, Some(&session.id)).await;
            return Err(ApiError::BadRequest("this purchase was cancelled".to_owned()));
        }
        // If the session is paid anyway, the webhook finds the purchase by the id in the
        // session's metadata.
        Err(e) => {
            abandon_checkout(&state, purchase.id, Some(&session.id)).await;
            return Err(e.into());
        }
    }
    log::info!(
        "user {} started purchase {} of item {} for {} {}",
        user.0.id, purchase.id, item.id, purchase.price, purchase.currency
    );
    Ok(HttpResponse::Created().json(Checkout { purchase_id: purchase.id, url }))
}

/// At most this many purchases may be started per buyer in any `CHECKOUT_WINDOW_SECS`.
const CHECKOUT_BURST: i64 = 10;
const CHECKOUT_WINDOW_SECS: i64 = 10 * 60;

const FREE_ITEM: &str =
    "this item is currently free, which the payment provider can't process; ask a store manager to adjust the discount";
const NOT_GRANTABLE: &str =
    "this item can't be bought right now: it grants a usergroup its store no longer allows";

/// Best-effort cleanup of a checkout that failed after its purchase was started: expires
/// the Stripe session, if one was opened, then cancels the purchase, returning its stock.
/// Failures are logged; the stale-purchase sweeper and the webhook settle what's left.
async fn abandon_checkout(state: &AppState, purchase_id: i64, session_id: Option<&str>) {
    if let (Some(stripe), Some(session)) = (&state.stripe, session_id)
        && let Err(e) = stripe.expire_checkout_session(session).await
    {
        log::error!("failed to expire checkout session {session} of abandoned purchase {purchase_id}: {e}");
    }
    if let Err(e) = Purchase::cancel(&state.db, purchase_id).await {
        log::error!("failed to cancel abandoned purchase {purchase_id}: {e}");
    }
}

/// The cancel link's proof that it came from the purchase's Checkout Session: the hex
/// HMAC of the purchase id. Without it, any site could send a logged-in buyer to the cancel
/// link and close their checkout.
fn cancel_token(secret: &str, purchase_id: i64) -> String {
    cancel_mac(secret, purchase_id).finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn cancel_mac(secret: &str, purchase_id: i64) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(format!("store-cancel:{purchase_id}").as_bytes());
    mac
}

/// Compares in constant time.
fn valid_cancel_token(secret: &str, purchase_id: i64, token: &str) -> bool {
    decode_hex(token).is_some_and(|token| cancel_mac(secret, purchase_id).verify_slice(&token).is_ok())
}

/// Closes a pending purchase's Checkout Session and cancels the purchase, returning its
/// stock. A session that turns out to have completed is left for the webhook to settle (it
/// was paid, or a delayed payment is clearing), and `None` is returned; so a paid purchase
/// is never cancelled. Otherwise returns the purchase as it now stands.
async fn release_pending(state: &AppState, purchase: &Purchase) -> Result<Option<Purchase>, ApiError> {
    if let (Some(stripe), Some(session)) = (&state.stripe, &purchase.stripe_checkout_session_id) {
        match stripe.expire_checkout_session(session).await {
            Ok(_) => {}
            // Already completed or expired. Only an expired one is ours to cancel.
            Err(ApiError::StripeRejected(e)) => {
                log::debug!("couldn't expire checkout session {session}: {e}");
                let current = stripe.retrieve_checkout_session(session).await?;
                if current.status.as_deref() == Some("complete") {
                    log::info!("purchase {} not cancelled: its checkout session {session} completed", purchase.id);
                    return Ok(None);
                }
            }
            Err(e) => return Err(e),
        }
    }
    Ok(Some(Purchase::cancel(&state.db, purchase.id).await?.unwrap_or_else(|| purchase.clone())))
}

#[derive(Deserialize)]
struct CancelBody {
    /// From the cancel link; see [`cancel_token`].
    token: String,
}

/// Cancels the buyer's own pending purchase (they backed out of Checkout), returning its
/// stock and closing the Stripe session; see [`release_pending`]. Needs the token from the
/// cancel link. Returns the purchase; 404 for other people's or with a wrong token.
#[post("/store/purchases/{purchase_id}/cancel")]
async fn cancel_purchase(
    state: web::Data<AppState>,
    user: AuthUser,
    purchase_id: web::Path<i64>,
    body: web::Json<CancelBody>,
) -> Result<HttpResponse, ApiError> {
    let purchase_id = purchase_id.into_inner();
    if !valid_cancel_token(&state.config.jwt_secret, purchase_id, body.token.trim()) {
        return Err(ApiError::NotFound("purchase"));
    }
    let purchase = Purchase::find_by_id(&state.db, purchase_id)
        .await?
        .filter(|p| p.buyer_id == user.0.id)
        .ok_or(ApiError::NotFound("purchase"))?;
    if purchase.status != PurchaseStatus::Pending {
        return Ok(HttpResponse::Ok().json(purchase));
    }
    let purchase = release_pending(&state, &purchase).await?.unwrap_or(purchase);
    Ok(HttpResponse::Ok().json(purchase))
}

/// The buyer's receipts with their delivery status, newest first.
#[get("/store/purchases")]
async fn my_purchases(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    let mut receipts: Vec<ReceiptView> = Receipt::list_for_buyer(&state.db, user.0.id).await?;
    // The addon's error text is for staff; buyers see the status.
    for receipt in &mut receipts {
        if let Some(fulfillment) = &mut receipt.fulfillment {
            fulfillment.last_error = None;
        }
    }
    Ok(HttpResponse::Ok().json(receipts))
}

/// Stripe's webhook. The signature is always verified; unsigned requests are rejected.
/// Every handler is idempotent, so Stripe may redeliver events freely.
#[post("/stripe/webhook")]
async fn webhook(state: web::Data<AppState>, req: HttpRequest, body: web::Bytes) -> Result<HttpResponse, ApiError> {
    let stripe = state.stripe()?;
    let signature = req
        .headers()
        .get("Stripe-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("missing Stripe-Signature header".to_owned()))?;
    let event = stripe.verify_webhook(&body, signature, Utc::now().timestamp())?;
    handle_event(&state, &event).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "received": true })))
}

async fn handle_event(state: &AppState, event: &Event) -> Result<(), ApiError> {
    let object = &event.data.object;
    let str_field = |key: &str| object.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    match event.kind.as_str() {
        "checkout.session.completed" | "checkout.session.async_payment_succeeded" => {
            let session = str_field("id").ok_or_else(|| ApiError::BadRequest("event has no session id".to_owned()))?;
            // With delayed payment methods the session completes before it's paid; the
            // `async_payment_succeeded` event follows once it is.
            let paid = matches!(str_field("payment_status").as_deref(), Some("paid" | "no_payment_required"));
            if !paid {
                log::info!("checkout session {session} completed but not paid yet ({})", event.id);
                return Ok(());
            }
            let payment_intent = str_field("payment_intent").unwrap_or_else(|| format!("session:{session}"));
            // Sessions are opened with the purchase id in their metadata (and as the client
            // reference), which finds a purchase whose session id was never recorded.
            let purchase_id = object
                .get("metadata")
                .and_then(|m| m.get("purchase_id"))
                .and_then(|v| v.as_str())
                .or_else(|| object.get("client_reference_id").and_then(|v| v.as_str()))
                .and_then(|id| id.parse::<i64>().ok());
            // What Stripe charged before tax, to check against what the purchase recorded.
            let charged = object
                .get("amount_subtotal")
                .and_then(|v| v.as_i64())
                .zip(str_field("currency"));
            record_paid(state, &session, purchase_id, &payment_intent, charged, &event.id).await?;
        }
        "checkout.session.expired" | "checkout.session.async_payment_failed" => {
            let session = str_field("id").ok_or_else(|| ApiError::BadRequest("event has no session id".to_owned()))?;
            if let Some(purchase) = Purchase::find_by_session(&state.db, &session).await?
                && let Some(cancelled) = Purchase::cancel(&state.db, purchase.id).await?
            {
                log::info!("purchase {} cancelled: checkout session {session} {}", cancelled.id, event.kind);
            }
        }
        "charge.refunded" => {
            // Partial refunds leave the receipt paid; `refunded` is true once it's refunded in full.
            if object.get("refunded").and_then(|v| v.as_bool()) == Some(true) {
                settle(state, str_field("payment_intent"), ReceiptStatus::Refunded, &event.kind).await?;
            }
        }
        "charge.dispute.created" => {
            settle(state, str_field("payment_intent"), ReceiptStatus::Disputed, &event.kind).await?;
        }
        // A disputed receipt stays DISPUTED and its rewards revoked even if the dispute is
        // won: whether to deliver them again is left to staff.
        "charge.dispute.closed" if str_field("status").as_deref() == Some("won") => {
            log::warn!(
                "dispute {} on payment {} was won; its receipt stays disputed and its rewards revoked until staff act ({})",
                str_field("id").unwrap_or_default(),
                str_field("payment_intent").unwrap_or_default(),
                event.id
            );
        }
        other => log::debug!("ignoring stripe event {} ({other})", event.id),
    }
    Ok(())
}

/// Turns the paid Checkout Session `session` into a receipt and fulfillment (see
/// [`Purchase::mark_paid`]) and tells the buyer. `charged` is the pre-tax amount and
/// currency Stripe reports, if known; a mismatch with the purchase is logged for staff.
/// `source` (the webhook event, or the sweeper) is for the logs.
async fn record_paid(
    state: &AppState,
    session: &str,
    purchase_id: Option<i64>,
    payment_intent: &str,
    charged: Option<(i64, String)>,
    source: &str,
) -> Result<(), ApiError> {
    match Purchase::mark_paid(&state.db, session, purchase_id, payment_intent).await? {
        MarkPaid::Paid(receipt, fulfillment) => {
            log::info!(
                "purchase paid: receipt {} for user {} ({} {}), fulfillment {} for server {} ({source})",
                receipt.id, receipt.buyer_id, receipt.price, receipt.currency, fulfillment.id, fulfillment.server_id
            );
            if let Some((amount, currency)) = charged
                && (amount != receipt.price || !currency.eq_ignore_ascii_case(&receipt.currency))
            {
                log::error!(
                    "checkout session {session} charged {amount} {currency} but receipt {} records {} {} ({source})",
                    receipt.id, receipt.price, receipt.currency
                );
            }
            notify::purchase_paid(&state.db, &receipt).await;
        }
        MarkPaid::AlreadyPaid => log::debug!("checkout session {session} already handled ({source})"),
        // Ours, but the purchase is gone (its server was deleted): the buyer paid for
        // nothing. Retrying wouldn't bring it back, so this is for staff.
        MarkPaid::Unknown if purchase_id.is_some() => log::error!(
            "checkout session {session} was paid ({payment_intent}) but purchase {} no longer exists; refund it from the Stripe dashboard ({source})",
            purchase_id.unwrap_or_default()
        ),
        // Not a GarryNet session (e.g. `stripe trigger`, or another integration).
        MarkPaid::Unknown => log::debug!("ignoring checkout session {session} without a purchase ({source})"),
    }
    Ok(())
}

/// A completed session whose delayed payment (e.g. a bank debit) hasn't cleared keeps its
/// purchase pending, and its stock reserved, for at most this long.
const ASYNC_PAYMENT_DAYS: i64 = 14;

/// Settles pending purchases older than `minutes`, whose Checkout Session expired or was
/// paid without a webhook reaching us, by asking Stripe how each session stands: paid ones
/// are recorded as paid, ones whose delayed payment is still clearing are left (for up to
/// [`ASYNC_PAYMENT_DAYS`]), and the rest are cancelled, returning their stock. A cancelled
/// purchase that's paid after all is reinstated by the webhook. Returns how many were
/// cancelled and how many recorded as paid. Stops early if Stripe is unreachable.
pub async fn sweep_stale_purchases(state: &AppState, minutes: i64) -> Result<(usize, usize), ApiError> {
    let (mut cancelled, mut paid) = (0, 0);
    for purchase in Purchase::stale_pending(&state.db, minutes).await? {
        let (Some(stripe), Some(session)) = (&state.stripe, purchase.stripe_checkout_session_id.as_deref()) else {
            // Never got a session, or Stripe isn't configured: nothing can pay it.
            cancelled += usize::from(Purchase::cancel(&state.db, purchase.id).await?.is_some());
            continue;
        };
        let current = match stripe.retrieve_checkout_session(session).await {
            Ok(current) => current,
            Err(ApiError::StripeRejected(e)) => {
                log::warn!("stale purchase {}: Stripe doesn't know checkout session {session} ({e}); cancelling", purchase.id);
                cancelled += usize::from(Purchase::cancel(&state.db, purchase.id).await?.is_some());
                continue;
            }
            Err(e) => {
                log::warn!("stale purchase sweep stopped: {e}");
                break;
            }
        };
        match (current.status.as_deref(), current.payment_status.as_deref()) {
            (Some("complete"), Some("paid" | "no_payment_required")) => {
                let payment_intent = current.payment_intent.unwrap_or_else(|| format!("session:{session}"));
                record_paid(state, session, Some(purchase.id), &payment_intent, None, "stale purchase sweep").await?;
                paid += 1;
            }
            (Some("complete"), _) if purchase.created_at > Utc::now() - Duration::days(ASYNC_PAYMENT_DAYS) => {}
            (status, _) => {
                if status == Some("open")
                    && let Err(e) = stripe.expire_checkout_session(session).await
                {
                    log::warn!("couldn't expire stale checkout session {session}: {e}");
                }
                cancelled += usize::from(Purchase::cancel(&state.db, purchase.id).await?.is_some());
            }
        }
    }
    Ok((cancelled, paid))
}

/// Moves the receipt of `payment_intent` to `status`, revoking its fulfillment.
async fn settle(state: &AppState, payment_intent: Option<String>, status: ReceiptStatus, kind: &str) -> Result<(), ApiError> {
    let Some(payment_intent) = payment_intent else {
        log::warn!("stripe {kind} event without a payment intent");
        return Ok(());
    };
    let Some(receipt) = Receipt::find_by_payment_intent(&state.db, &payment_intent).await? else {
        log::warn!("stripe {kind} for unknown payment intent {payment_intent}");
        return Ok(());
    };
    if let Some((receipt, fulfillment)) = Receipt::set_status(&state.db, receipt.id, status).await? {
        log::info!(
            "receipt {} is now {:?} ({kind}); fulfillment {} revoked",
            receipt.id,
            receipt.status,
            fulfillment.map_or("none".to_owned(), |f| f.id.to_string())
        );
        // Disputes aren't a user notification; staff handle them from the admin pages.
        if receipt.status == ReceiptStatus::Refunded {
            notify::purchase_refunded(&state.db, &receipt, None).await;
        }
    }
    Ok(())
}

/// An enabled store and its server; 404 for disabled or missing ones.
async fn find_enabled_store(state: &AppState, id: i64) -> Result<(StoreSummary, Server), ApiError> {
    let store = ItemStore::find_by_id(&state.db, id)
        .await?
        .filter(|s| s.store.enabled)
        .ok_or(ApiError::NotFound("store"))?;
    let server = Server::find_by_id(&state.db, store.store.server_id)
        .await?
        .ok_or(ApiError::NotFound("store"))?;
    Ok((store, server))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_tokens() {
        let secret = "a-secret-that-is-at-least-32-bytes-long";
        let token = cancel_token(secret, 42);
        assert_eq!(token.len(), 64);
        assert!(valid_cancel_token(secret, 42, &token));
        assert!(!valid_cancel_token(secret, 43, &token));
        assert!(!valid_cancel_token("another-secret-at-least-32-bytes-long", 42, &token));
        assert!(!valid_cancel_token(secret, 42, ""));
        assert!(!valid_cancel_token(secret, 42, "zz"));
    }
}
