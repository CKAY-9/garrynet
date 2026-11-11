//! Managing the store (docs/STORE.md): stores, items and discounts need Admin Access and
//! Store Management; a store's grantable usergroups need Elevated Control on top; receipts
//! and refunds need Admin Access and Store Finance.

use actix_web::{HttpResponse, delete, get, post, put, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Server,
        store::{
            DiscountDetails, Fulfillment, ItemDetails, ItemStore, PricedItem, Receipt, ReceiptFilter, ReceiptStatus,
            ReceiptView, Reward, StoreDetails, StoreDiscount, StoreItem, StoreSummary,
        },
    },
    notify,
    routes::{
        markdown::markdown,
        store::with_servers,
        validate::{bad, optional_url, required_text, text},
    },
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    // `receipts` before `{store_id}`, so it isn't matched as a store id.
    cfg.service(list_stores)
        .service(create_store)
        .service(list_receipts)
        .service(refund)
        .service(retry_fulfillment)
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
    if current.store.currency != details.currency && current.item_count > 0 {
        return Err(bad(format!(
            "the currency can't change while the store has items (their prices are in {}); archive them first",
            current.store.currency.to_ascii_uppercase()
        )));
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
}

/// Receipts, newest first (at most 500). Requires Admin Access and Store Finance.
#[get("/admin/store/receipts")]
async fn list_receipts(
    state: web::Data<AppState>,
    user: AuthUser,
    query: web::Query<ReceiptQuery>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, FINANCE).await?;
    let filter = parse_receipt_filter(query.into_inner())?;
    let receipts: Vec<ReceiptView> = Receipt::list(&state.db, &filter).await?;
    Ok(HttpResponse::Ok().json(receipts))
}

/// Refunds a receipt in full through Stripe and revokes its fulfillment. Requires Admin
/// Access and Store Finance. 400 unless the receipt is still paid.
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
    let receipt = view.receipt;
    if receipt.status != ReceiptStatus::Paid {
        return Err(bad(format!("receipt is already {}", format!("{:?}", receipt.status).to_lowercase())));
    }
    // Sessions that needed no payment are recorded with a placeholder instead of a
    // PaymentIntent; there's nothing to refund through Stripe.
    if receipt.stripe_payment_intent_id.starts_with("session:") {
        return Err(bad("this receipt has no payment to refund".to_owned()));
    }
    // `None` if it was already refunded in full (e.g. from the Stripe dashboard, with the
    // webhook not yet arrived); the receipt is still marked refunded below.
    let refund = stripe
        .refund(&receipt.stripe_payment_intent_id, &format!("refund-receipt-{}", receipt.id))
        .await?;
    // The `charge.refunded` webhook does the same later; it finds nothing left to change
    // (or, if it got here first, we don't), so the buyer is notified once.
    if let Some((refunded, _)) = Receipt::set_status(&state.db, receipt.id, ReceiptStatus::Refunded).await? {
        notify::purchase_refunded(&state.db, &refunded, Some(user.0.id)).await;
    }
    log::info!(
        "user {} refunded receipt {} ({} {}) for user {}: stripe refund {} ({})",
        user.0.id, receipt.id, receipt.price, receipt.currency, receipt.buyer_id,
        refund.as_ref().map_or("none", |r| r.id.as_str()),
        refund.as_ref().and_then(|r| r.status.as_deref()).unwrap_or("already refunded")
    );
    let updated = Receipt::find_by_id(&state.db, receipt.id)
        .await?
        .ok_or(ApiError::NotFound("receipt"))?;
    Ok(HttpResponse::Ok().json(updated))
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
    })
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
    Ok(ReceiptFilter {
        status,
        store_id: id("store", query.store)?,
        buyer_id: id("buyer", query.buyer)?,
        limit: MAX_RECEIPTS,
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
        })
        .unwrap_or_else(|_| panic!("valid store rejected"));
        assert_eq!(d.currency, "eur");
        assert_eq!(d.override_name, None);
        assert_eq!(d.store_message.as_deref(), Some("Hi"));
        assert!(normalize_store(StoreBody { enabled: true, currency: Some("dollars".into()), override_name: None, store_message: None }).is_err());
        assert_eq!(normalize_store(StoreBody { enabled: false, currency: None, override_name: None, store_message: None }).unwrap().currency, "usd");
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

        let f = parse_receipt_filter(ReceiptQuery { status: Some("refunded".into()), store: Some("3".into()), buyer: Some("".into()) }).unwrap();
        assert_eq!(f.status, Some(ReceiptStatus::Refunded));
        assert_eq!(f.store_id, Some(3));
        assert_eq!(f.buyer_id, None);
        assert!(parse_receipt_filter(ReceiptQuery { status: Some("pending".into()), store: None, buyer: None }).is_err());
        assert!(parse_receipt_filter(ReceiptQuery { status: None, store: Some("x".into()), buyer: None }).is_err());
        assert!(normalize_usergroups(vec!["vip".into(), " vip".into(), "donor".into()]).unwrap().len() == 2);
        assert!(normalize_usergroups(vec!["super admin".into()]).is_err());
    }
}
