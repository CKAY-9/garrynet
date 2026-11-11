//! The store (docs/STORE.md): one item store per server, whose items grant in-game rewards.
//! A purchase is a Stripe Checkout Session; once paid it leaves a receipt and a fulfillment
//! for the server's addon to deliver.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder, types::Json};

use super::support::UserRef;

/// What an item gives the buyer in-game.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Reward {
    /// A Garry's Mod usergroup, permanent unless `duration_seconds` is set. Must be one of
    /// the store's `grantable_usergroups`.
    Usergroup {
        usergroup: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_seconds: Option<i64>,
    },
    /// DarkRP servers only.
    DarkrpMoney { amount: i64 },
    /// A permanent weapon, by class.
    Weapon { class: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "purchase_status", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PurchaseStatus {
    /// The Checkout Session is open.
    Pending,
    Paid,
    /// The session expired or the buyer backed out; reserved stock was returned.
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "receipt_status", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptStatus {
    Paid,
    Refunded,
    Disputed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "fulfillment_status", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FulfillmentStatus {
    /// Waiting for the server's addon to deliver the rewards.
    Pending,
    Fulfilled,
    /// The addon couldn't apply the rewards; it retries up to [`MAX_ATTEMPTS`].
    Failed,
    /// The receipt was refunded or disputed. If the rewards were delivered, the addon takes
    /// them back on its next sync.
    Revoked,
}

/// Failed fulfillments are offered to the addon again until they've been tried this often.
pub const MAX_ATTEMPTS: i32 = 20;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ItemStore {
    pub id: i64,
    /// One store per server.
    pub server_id: i64,
    /// Hidden from `/store` and nothing can be bought while false.
    pub enabled: bool,
    /// ISO 4217, lowercase; applies to every item.
    pub currency: String,
    /// Named after the server unless set.
    pub override_name: Option<String>,
    /// Shown at the top of the store page.
    pub store_message: Option<String>,
    /// Garry's Mod usergroups items may grant. Only Elevated Control changes it.
    pub grantable_usergroups: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// A store as listed, with how much is in it and the store-wide discount in force.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct StoreSummary {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub store: ItemStore,
    /// Items that aren't archived.
    pub item_count: i64,
    /// Sum of the active store-wide discounts, 0 to 100.
    pub store_discount: i32,
}

/// Already validated (see `routes::store_admin`).
pub struct StoreDetails {
    pub enabled: bool,
    pub currency: String,
    pub override_name: Option<String>,
    pub store_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct StoreDiscount {
    pub id: i64,
    pub store_id: i64,
    /// Percent off, added to each item's own discount.
    pub discount: i32,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
    pub starts_at: Option<DateTime<Utc>>,
    /// Indefinite when `None`.
    pub ends_at: Option<DateTime<Utc>>,
    /// Whether now is within `starts_at`/`ends_at`. Computed when read.
    pub active: bool,
}

/// Already validated (see `routes::store_admin`).
pub struct DiscountDetails {
    pub discount: i32,
    pub message: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct StoreItem {
    pub id: i64,
    pub store_id: i64,
    pub name: String,
    /// Plain text.
    pub short_description: String,
    /// Markdown.
    pub full_description: String,
    pub image_url: Option<String>,
    /// Ascending.
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
    /// Listed but not buyable while false.
    pub available: bool,
    /// Soft-deleted: hidden everywhere, kept for receipts and fulfillments.
    pub archived: bool,
    /// Minor units of the store's currency.
    pub price: i64,
    /// Percent off, before store-wide discounts.
    pub discount: i32,
    /// Limited releases; `None` is unlimited.
    pub starting_stock: Option<i32>,
    /// Excludes stock reserved by pending purchases.
    pub remaining_stock: Option<i32>,
    /// Purchases (paid or pending) one user may make; `None` is unlimited.
    pub max_per_user: Option<i32>,
    /// Incremented when a purchase is paid.
    pub all_time_purchases: i32,
    pub rewards: Json<Vec<Reward>>,
}

/// An item with its price after every discount, as shown to buyers.
#[derive(Clone, Debug, Serialize)]
pub struct PricedItem {
    #[serde(flatten)]
    pub item: StoreItem,
    /// The item's discount plus the store's active discounts, capped at 100.
    pub total_discount: i32,
    /// Minor units, after `total_discount`.
    pub final_price: i64,
}

/// Already validated (see `routes::store_admin`).
pub struct ItemDetails {
    pub name: String,
    pub short_description: String,
    pub full_description: String,
    pub image_url: Option<String>,
    pub sort_order: i32,
    pub available: bool,
    pub price: i64,
    pub discount: i32,
    pub starting_stock: Option<i32>,
    pub max_per_user: Option<i32>,
    pub rewards: Vec<Reward>,
}

/// A checkout attempt; see [`PurchaseStatus`].
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Purchase {
    pub id: i64,
    pub store_id: i64,
    pub item_id: i64,
    pub buyer_id: i64,
    /// The buyer's SteamID64 at purchase time.
    pub steam_id: String,
    pub status: PurchaseStatus,
    pub stripe_checkout_session_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub quantity: i32,
    pub list_price: i64,
    pub discount_percentage: i32,
    pub price: i64,
    pub currency: String,
    /// The item's rewards when the buyer checked out.
    pub rewards: Json<Vec<Reward>>,
    /// Whether a unit of limited stock was taken for this purchase.
    pub stock_reserved: bool,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Receipt {
    pub id: i64,
    pub stripe_checkout_session_id: String,
    pub stripe_payment_intent_id: String,
    pub buyer_id: i64,
    /// `None` once the item has been deleted along with its server.
    pub item_id: Option<i64>,
    pub item_name: String,
    pub store_id: Option<i64>,
    pub server_id: Option<i64>,
    pub store_name: String,
    pub quantity: i32,
    pub status: ReceiptStatus,
    pub purchased_at: DateTime<Utc>,
    /// Minor units, before discounts.
    pub list_price: i64,
    pub discount_percentage: i32,
    /// Minor units, what was charged (before any tax Stripe added).
    pub price: i64,
    pub currency: String,
    pub status_changed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Fulfillment {
    pub id: i64,
    pub receipt_id: i64,
    pub buyer_id: i64,
    pub steam_id: String,
    pub item_id: Option<i64>,
    pub server_id: i64,
    /// Snapshot at purchase time; later item edits don't change it.
    pub rewards: Json<Vec<Reward>>,
    pub status: FulfillmentStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub fulfilled_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    /// When the addon confirmed it took back the rewards of a revoked fulfillment.
    pub revocation_applied_at: Option<DateTime<Utc>>,
}

/// A receipt with who bought it and how delivery is going.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ReceiptView {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub receipt: Receipt,
    pub buyer: Json<UserRef>,
    pub fulfillment: Option<Json<Fulfillment>>,
}

/// Filters for the finance receipt list. All optional; together they must all match.
#[derive(Debug, Default)]
pub struct ReceiptFilter {
    pub status: Option<ReceiptStatus>,
    pub store_id: Option<i64>,
    pub buyer_id: Option<i64>,
    pub limit: i64,
}

/// Revenue in one currency.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Income {
    pub currency: String,
    /// Minor units.
    pub amount: i64,
}

/// What the addon reports back about a fulfillment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckResult {
    /// The rewards were applied.
    Fulfilled,
    /// They couldn't be; `error` says why. Retried later.
    Failed,
    /// The rewards of a revoked fulfillment were taken back.
    Revoked,
}

/// The outcome of starting a purchase.
#[allow(clippy::large_enum_variant, reason = "returned once and matched at once")]
pub enum Begin {
    /// The purchase, priced and with its currency as of the moment it was made, and the
    /// item as it stood then.
    Started { purchase: Purchase, item: StoreItem },
    /// The item is archived, unavailable or its store is disabled.
    Unavailable,
    SoldOut,
    /// The buyer has reached the item's `max_per_user`.
    LimitReached,
    /// The item's discounts bring it to nothing, which Stripe can't charge.
    Free,
    /// The item grants a usergroup its store no longer allows (`grantable_usergroups` was
    /// narrowed after the item was saved).
    NotGrantable(String),
    /// The buyer already has a checkout of this item open.
    AlreadyPending,
    /// The buyer has [`MAX_PENDING_PER_BUYER`] checkouts open.
    TooManyPending,
}

/// Checkouts one buyer may have open at once, across every store. Each holds a unit of
/// limited stock until it's paid or cancelled.
pub const MAX_PENDING_PER_BUYER: i64 = 3;

/// The outcome of a paid Checkout Session; see [`Purchase::mark_paid`].
#[allow(clippy::large_enum_variant, reason = "returned once and matched at once")]
pub enum MarkPaid {
    Paid(Receipt, Fulfillment),
    /// The purchase was already paid: a redelivered webhook.
    AlreadyPaid,
    /// No purchase has this session (or the purchase id in its metadata).
    Unknown,
}

/// The first usergroup `rewards` grant that isn't in `grantable`, if any.
pub fn ungrantable_usergroup<'a>(rewards: &'a [Reward], grantable: &[String]) -> Option<&'a str> {
    rewards.iter().find_map(|r| match r {
        Reward::Usergroup { usergroup, .. } if !grantable.contains(usergroup) => Some(usergroup.as_str()),
        _ => None,
    })
}

/// `price` after `total_discount` percent off, rounded to the nearest minor unit.
pub fn final_price(price: i64, total_discount: i32) -> i64 {
    let total = i64::from(total_discount.clamp(0, 100));
    (price * (100 - total) + 50) / 100
}

/// The item's discount plus the store's, capped at 100.
pub fn total_discount(item_discount: i32, store_discount: i32) -> i32 {
    (item_discount + store_discount).clamp(0, 100)
}

impl PricedItem {
    pub fn new(item: StoreItem, store_discount: i32) -> Self {
        let total_discount = total_discount(item.discount, store_discount);
        Self {
            final_price: final_price(item.price, total_discount),
            total_discount,
            item,
        }
    }
}

macro_rules! store_columns {
    ($s:literal) => {
        concat!(
            $s, ".id, ", $s, ".server_id, ", $s, ".enabled, ", $s, ".currency, ",
            $s, ".override_name, ", $s, ".store_message, ", $s, ".grantable_usergroups, ",
            $s, ".created_at, ", $s, ".last_updated"
        )
    };
}

/// Whether discount `d` is in force right now.
macro_rules! discount_active {
    () => {
        "(d.starts_at IS NULL OR d.starts_at <= now()) AND (d.ends_at IS NULL OR d.ends_at > now())"
    };
}

/// The sum of the active discounts of the store whose id is `$store`, capped at 100.
macro_rules! store_discount_sum {
    ($store:literal) => {
        concat!(
            "(SELECT LEAST(100, COALESCE(sum(d.discount), 0))::INTEGER FROM store_discounts d
              WHERE d.store_id = ", $store, " AND ", discount_active!(), ")"
        )
    };
}

/// Store columns plus `item_count` and `store_discount`, from `item_stores s`.
macro_rules! select_summary {
    ($rest:literal) => {
        concat!(
            "SELECT ", store_columns!("s"), ",
                    (SELECT count(*) FROM store_items i
                     WHERE i.store_id = s.id AND NOT i.archived) AS item_count, ",
            store_discount_sum!("s.id"), " AS store_discount
             FROM item_stores s ",
            $rest
        )
    };
}

/// Discount columns plus `active`, from `store_discounts d`.
macro_rules! select_discount {
    ($rest:literal) => {
        concat!(
            "SELECT d.id, d.store_id, d.discount, d.message, d.created_at, d.last_updated,
                    d.starts_at, d.ends_at, ",
            discount_active!(),
            " AS active FROM store_discounts d ",
            $rest
        )
    };
}

/// The buyer (`$1`)'s purchases of an item (`$2`) that count towards its `max_per_user`:
/// pending and paid ones, except paid ones whose receipt was refunded.
const COUNT_FOR_BUYER: &str = "
    SELECT count(*) FROM store_purchases p
    LEFT JOIN store_receipts r ON r.stripe_checkout_session_id = p.stripe_checkout_session_id
    WHERE p.buyer_id = $1 AND p.item_id = $2 AND p.status IN ('PENDING', 'PAID')
      AND r.status IS DISTINCT FROM 'REFUNDED'";

impl ItemStore {
    /// Every store, enabled or not, oldest first.
    pub async fn list(db: &PgPool) -> sqlx::Result<Vec<StoreSummary>> {
        sqlx::query_as(select_summary!("ORDER BY s.created_at, s.id"))
            .fetch_all(db)
            .await
    }

    /// The stores players can see.
    pub async fn list_enabled(db: &PgPool) -> sqlx::Result<Vec<StoreSummary>> {
        sqlx::query_as(select_summary!("WHERE s.enabled ORDER BY s.created_at, s.id"))
            .fetch_all(db)
            .await
    }

    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<StoreSummary>> {
        sqlx::query_as(select_summary!("WHERE s.id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// Creates the server's store. `None` if the server already has one.
    pub async fn create(db: &PgPool, server_id: i64, details: &StoreDetails) -> sqlx::Result<Option<ItemStore>> {
        sqlx::query_as(concat!(
            "INSERT INTO item_stores (server_id, enabled, currency, override_name, store_message)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (server_id) DO NOTHING
             RETURNING ", store_columns!("item_stores")
        ))
        .bind(server_id)
        .bind(details.enabled)
        .bind(&details.currency)
        .bind(&details.override_name)
        .bind(&details.store_message)
        .fetch_optional(db)
        .await
    }

    /// Replaces the store's settings. `None` if there is no such store.
    pub async fn update(db: &PgPool, id: i64, details: &StoreDetails) -> sqlx::Result<Option<ItemStore>> {
        sqlx::query_as(concat!(
            "UPDATE item_stores
                SET enabled = $2, currency = $3, override_name = $4, store_message = $5,
                    last_updated = now()
              WHERE id = $1
              RETURNING ", store_columns!("item_stores")
        ))
        .bind(id)
        .bind(details.enabled)
        .bind(&details.currency)
        .bind(&details.override_name)
        .bind(&details.store_message)
        .fetch_optional(db)
        .await
    }

    /// Replaces the usergroups items may grant, and makes unavailable every item that
    /// grants a usergroup no longer allowed, so it can't be bought until a manager fixes
    /// it. Returns the store and how many items were made unavailable; `None` if there is
    /// no such store. Callers check for Elevated Control.
    pub async fn set_grantable_usergroups(
        db: &PgPool,
        id: i64,
        usergroups: &[String],
    ) -> sqlx::Result<Option<(ItemStore, u64)>> {
        let mut tx = db.begin().await?;
        let store: Option<ItemStore> = sqlx::query_as(concat!(
            "UPDATE item_stores SET grantable_usergroups = $2, last_updated = now()
              WHERE id = $1
              RETURNING ", store_columns!("item_stores")
        ))
        .bind(id)
        .bind(usergroups)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(store) = store else {
            return Ok(None);
        };
        let disabled = sqlx::query(
            "UPDATE store_items SET available = false, last_updated = now()
              WHERE store_id = $1 AND available AND NOT archived
                AND EXISTS (SELECT 1 FROM jsonb_array_elements(rewards) r
                            WHERE r->>'type' = 'usergroup' AND NOT (r->>'usergroup' = ANY($2)))",
        )
        .bind(id)
        .bind(usergroups)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        tx.commit().await?;
        Ok(Some((store, disabled)))
    }
}

impl StoreDiscount {
    /// Every discount of the store, newest first.
    pub async fn list(db: &PgPool, store_id: i64) -> sqlx::Result<Vec<StoreDiscount>> {
        sqlx::query_as(select_discount!("WHERE d.store_id = $1 ORDER BY d.created_at DESC, d.id DESC"))
            .bind(store_id)
            .fetch_all(db)
            .await
    }

    /// The store's discounts in force right now.
    pub async fn list_active(db: &PgPool, store_id: i64) -> sqlx::Result<Vec<StoreDiscount>> {
        sqlx::query_as(select_discount!(
            "WHERE d.store_id = $1 AND (d.starts_at IS NULL OR d.starts_at <= now())
               AND (d.ends_at IS NULL OR d.ends_at > now())
             ORDER BY d.created_at DESC, d.id DESC"
        ))
        .bind(store_id)
        .fetch_all(db)
        .await
    }

    pub async fn create(db: &PgPool, store_id: i64, details: &DiscountDetails) -> sqlx::Result<StoreDiscount> {
        // A CTE so the computed `active` can be selected from the inserted row.
        sqlx::query_as(
            "WITH d AS (INSERT INTO store_discounts (store_id, discount, message, starts_at, ends_at)
                        VALUES ($1, $2, $3, $4, $5) RETURNING *)
             SELECT d.id, d.store_id, d.discount, d.message, d.created_at, d.last_updated,
                    d.starts_at, d.ends_at,
                    (d.starts_at IS NULL OR d.starts_at <= now()) AND (d.ends_at IS NULL OR d.ends_at > now())
                    AS active
             FROM d",
        )
        .bind(store_id)
        .bind(details.discount)
        .bind(&details.message)
        .bind(details.starts_at)
        .bind(details.ends_at)
        .fetch_one(db)
        .await
    }

    /// `None` if the store has no such discount.
    pub async fn update(db: &PgPool, store_id: i64, id: i64, details: &DiscountDetails) -> sqlx::Result<Option<StoreDiscount>> {
        let updated = sqlx::query(
            "UPDATE store_discounts
                SET discount = $3, message = $4, starts_at = $5, ends_at = $6, last_updated = now()
              WHERE id = $1 AND store_id = $2",
        )
        .bind(id)
        .bind(store_id)
        .bind(details.discount)
        .bind(&details.message)
        .bind(details.starts_at)
        .bind(details.ends_at)
        .execute(db)
        .await?
        .rows_affected();
        if updated == 0 {
            return Ok(None);
        }
        sqlx::query_as(select_discount!("WHERE d.id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// False if the store has no such discount.
    pub async fn delete(db: &PgPool, store_id: i64, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM store_discounts WHERE id = $1 AND store_id = $2")
            .bind(id)
            .bind(store_id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

impl StoreItem {
    /// The store's items that aren't archived, in display order.
    pub async fn list(db: &PgPool, store_id: i64) -> sqlx::Result<Vec<StoreItem>> {
        sqlx::query_as(
            "SELECT * FROM store_items WHERE store_id = $1 AND NOT archived ORDER BY sort_order, id",
        )
        .bind(store_id)
        .fetch_all(db)
        .await
    }

    /// An item of the store, unless it's archived.
    pub async fn find(db: &PgPool, store_id: i64, id: i64) -> sqlx::Result<Option<StoreItem>> {
        sqlx::query_as("SELECT * FROM store_items WHERE id = $1 AND store_id = $2 AND NOT archived")
            .bind(id)
            .bind(store_id)
            .fetch_optional(db)
            .await
    }

    pub async fn create(db: &PgPool, store_id: i64, d: &ItemDetails) -> sqlx::Result<StoreItem> {
        sqlx::query_as(
            "INSERT INTO store_items
                (store_id, name, short_description, full_description, image_url, sort_order,
                 available, price, discount, starting_stock, remaining_stock, max_per_user, rewards)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10, $11, $12)
             RETURNING *",
        )
        .bind(store_id)
        .bind(&d.name)
        .bind(&d.short_description)
        .bind(&d.full_description)
        .bind(&d.image_url)
        .bind(d.sort_order)
        .bind(d.available)
        .bind(d.price)
        .bind(d.discount)
        .bind(d.starting_stock)
        .bind(d.max_per_user)
        .bind(Json(&d.rewards))
        .fetch_one(db)
        .await
    }

    /// Replaces the item's details. A change to `starting_stock` moves `remaining_stock` by
    /// the same amount (never below zero), so units already sold stay sold. `None` if the
    /// store has no such item or it's archived.
    pub async fn update(db: &PgPool, store_id: i64, id: i64, d: &ItemDetails) -> sqlx::Result<Option<StoreItem>> {
        sqlx::query_as(
            "UPDATE store_items
                SET name = $3, short_description = $4, full_description = $5, image_url = $6,
                    sort_order = $7, available = $8, price = $9, discount = $10,
                    remaining_stock = CASE
                        WHEN $11::INTEGER IS NULL THEN NULL
                        WHEN starting_stock IS NULL THEN $11
                        ELSE GREATEST(0, COALESCE(remaining_stock, 0) + ($11 - starting_stock))
                    END,
                    starting_stock = $11,
                    max_per_user = $12, rewards = $13, last_updated = now()
              WHERE id = $1 AND store_id = $2 AND NOT archived
              RETURNING *",
        )
        .bind(id)
        .bind(store_id)
        .bind(&d.name)
        .bind(&d.short_description)
        .bind(&d.full_description)
        .bind(&d.image_url)
        .bind(d.sort_order)
        .bind(d.available)
        .bind(d.price)
        .bind(d.discount)
        .bind(d.starting_stock)
        .bind(d.max_per_user)
        .bind(Json(&d.rewards))
        .fetch_optional(db)
        .await
    }

    /// Soft-deletes the item. False if the store has no such (unarchived) item.
    pub async fn archive(db: &PgPool, store_id: i64, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "UPDATE store_items SET archived = true, available = false, last_updated = now()
              WHERE id = $1 AND store_id = $2 AND NOT archived",
        )
        .bind(id)
        .bind(store_id)
        .execute(db)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}

/// How long a Checkout Session stays open. Stripe's minimum is 30 minutes from when it
/// receives the request, so this leaves room for latency and clock skew.
pub const CHECKOUT_TTL_MINUTES: i64 = 35;

impl Purchase {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Purchase>> {
        sqlx::query_as("SELECT * FROM store_purchases WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn find_by_session(db: &PgPool, session_id: &str) -> sqlx::Result<Option<Purchase>> {
        sqlx::query_as("SELECT * FROM store_purchases WHERE stripe_checkout_session_id = $1")
            .bind(session_id)
            .fetch_optional(db)
            .await
    }

    /// The buyer's purchases of an item that count towards its `max_per_user`: pending and
    /// paid ones, except refunded ones.
    pub async fn count_for_buyer(db: &PgPool, buyer_id: i64, item_id: i64) -> sqlx::Result<i64> {
        sqlx::query_scalar(COUNT_FOR_BUYER)
            .bind(buyer_id)
            .bind(item_id)
            .fetch_one(db)
            .await
    }

    /// Starts a purchase of one unit of the item, reserving a unit of limited stock. The item
    /// row is locked meanwhile, so two buyers can't take the last unit or exceed
    /// `max_per_user`, and its store row is share-locked, so its currency and grantable
    /// usergroups can't change underneath. The price (after every discount) and currency
    /// are read under those locks and recorded on the purchase, which is what the caller
    /// charges.
    pub async fn begin(db: &PgPool, item_id: i64, buyer_id: i64, steam_id: &str) -> sqlx::Result<Begin> {
        let mut tx = db.begin().await?;
        let item: Option<StoreItem> = sqlx::query_as(
            "SELECT i.* FROM store_items i JOIN item_stores s ON s.id = i.store_id
             WHERE i.id = $1 AND i.available AND NOT i.archived AND s.enabled
             FOR UPDATE OF i FOR SHARE OF s",
        )
        .bind(item_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(item) = item else {
            return Ok(Begin::Unavailable);
        };
        let (currency, grantable, store_discount): (String, Vec<String>, i32) = sqlx::query_as(concat!(
            "SELECT s.currency, s.grantable_usergroups, ", store_discount_sum!("s.id"),
            " FROM item_stores s WHERE s.id = $1"
        ))
        .bind(item.store_id)
        .fetch_one(&mut *tx)
        .await?;
        if let Some(group) = ungrantable_usergroup(&item.rewards, &grantable) {
            return Ok(Begin::NotGrantable(group.to_owned()));
        }
        let discount = total_discount(item.discount, store_discount);
        let price = final_price(item.price, discount);
        if price <= 0 {
            return Ok(Begin::Free);
        }
        // At most one open checkout per item, and a few overall, so one buyer can't hold all
        // of an item's limited stock (or many items') by opening checkouts they never pay.
        let (pending_item, pending_all): (i64, i64) = sqlx::query_as(
            "SELECT count(*) FILTER (WHERE item_id = $2), count(*) FROM store_purchases
             WHERE buyer_id = $1 AND status = 'PENDING'",
        )
        .bind(buyer_id)
        .bind(item_id)
        .fetch_one(&mut *tx)
        .await?;
        if pending_item > 0 {
            return Ok(Begin::AlreadyPending);
        }
        if pending_all >= MAX_PENDING_PER_BUYER {
            return Ok(Begin::TooManyPending);
        }
        if let Some(max) = item.max_per_user {
            let bought: i64 = sqlx::query_scalar(COUNT_FOR_BUYER)
                .bind(buyer_id)
                .bind(item_id)
                .fetch_one(&mut *tx)
                .await?;
            if bought >= i64::from(max) {
                return Ok(Begin::LimitReached);
            }
        }
        let reserved = match item.remaining_stock {
            None => false,
            Some(0) => return Ok(Begin::SoldOut),
            Some(_) => {
                sqlx::query("UPDATE store_items SET remaining_stock = remaining_stock - 1 WHERE id = $1")
                    .bind(item_id)
                    .execute(&mut *tx)
                    .await?;
                true
            }
        };
        let purchase: Purchase = sqlx::query_as(
            "INSERT INTO store_purchases
                (store_id, item_id, buyer_id, steam_id, list_price, discount_percentage, price,
                 currency, rewards, stock_reserved)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             RETURNING *",
        )
        .bind(item.store_id)
        .bind(item_id)
        .bind(buyer_id)
        .bind(steam_id)
        .bind(item.price)
        .bind(discount)
        .bind(price)
        .bind(&currency)
        .bind(&item.rewards)
        .bind(reserved)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Begin::Started { purchase, item })
    }

    /// The buyer's pending purchases of an item, oldest first.
    pub async fn pending_for_buyer(db: &PgPool, buyer_id: i64, item_id: i64) -> sqlx::Result<Vec<Purchase>> {
        sqlx::query_as(
            "SELECT * FROM store_purchases
             WHERE buyer_id = $1 AND item_id = $2 AND status = 'PENDING'
             ORDER BY created_at, id",
        )
        .bind(buyer_id)
        .bind(item_id)
        .fetch_all(db)
        .await
    }

    /// How many purchases the buyer started in the past `seconds`, and when the oldest of
    /// them was, for rate limiting checkout.
    pub async fn recent_by_buyer(db: &PgPool, buyer_id: i64, seconds: f64) -> sqlx::Result<(i64, Option<DateTime<Utc>>)> {
        sqlx::query_as(
            "SELECT count(*), min(created_at) FROM store_purchases
             WHERE buyer_id = $1 AND created_at > now() - make_interval(secs => $2)",
        )
        .bind(buyer_id)
        .bind(seconds)
        .fetch_one(db)
        .await
    }

    /// Records the Checkout Session opened for the purchase. `None` if the purchase isn't
    /// pending any more (the buyer cancelled it while the session was being created), in
    /// which case the session should be expired rather than handed out.
    pub async fn attach_session(db: &PgPool, id: i64, session_id: &str) -> sqlx::Result<Option<Purchase>> {
        sqlx::query_as(
            "UPDATE store_purchases SET stripe_checkout_session_id = $2
              WHERE id = $1 AND status = 'PENDING'
              RETURNING *",
        )
        .bind(id)
        .bind(session_id)
        .fetch_optional(db)
        .await
    }

    /// Cancels a pending purchase, returning its reserved stock. `None` if there is no such
    /// purchase or it isn't pending any more (so cancelling twice releases stock once).
    pub async fn cancel(db: &PgPool, id: i64) -> sqlx::Result<Option<Purchase>> {
        let mut tx = db.begin().await?;
        let purchase = cancel_pending(&mut tx, id).await?;
        tx.commit().await?;
        Ok(purchase)
    }

    /// Pending purchases older than `minutes`, oldest first: their Checkout Session should
    /// have been settled by a webhook by now. See `routes::store::sweep_stale_purchases`.
    pub async fn stale_pending(db: &PgPool, minutes: i64) -> sqlx::Result<Vec<Purchase>> {
        sqlx::query_as(
            "SELECT * FROM store_purchases
             WHERE status = 'PENDING' AND created_at < now() - make_interval(mins => $1)
             ORDER BY created_at, id",
        )
        .bind(minutes as i32)
        .fetch_all(db)
        .await
    }

    /// Marks the purchase of Checkout Session `session_id` paid: writes its receipt, bumps the
    /// item's `all_time_purchases` and creates the fulfillment. A redelivered webhook finds
    /// the purchase already paid and changes nothing.
    ///
    /// The purchase is found by its session, or failing that by `purchase_id` (from the
    /// session's metadata) if its session was never recorded, e.g. because the database
    /// failed right after Stripe created the session. A purchase cancelled for staleness that
    /// turns out to have been paid is reinstated (and its stock taken again, down to zero).
    pub async fn mark_paid(
        db: &PgPool,
        session_id: &str,
        purchase_id: Option<i64>,
        payment_intent: &str,
    ) -> sqlx::Result<MarkPaid> {
        let mut tx = db.begin().await?;
        let before: Option<Purchase> = sqlx::query_as(
            "SELECT * FROM store_purchases
             WHERE stripe_checkout_session_id = $1
                OR ($2::BIGINT IS NOT NULL AND id = $2 AND stripe_checkout_session_id IS NULL)
             ORDER BY (stripe_checkout_session_id IS NULL)
             LIMIT 1
             FOR UPDATE",
        )
        .bind(session_id)
        .bind(purchase_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(before) = before else {
            return Ok(MarkPaid::Unknown);
        };
        if before.status == PurchaseStatus::Paid {
            return Ok(MarkPaid::AlreadyPaid);
        }
        let purchase: Purchase = sqlx::query_as(
            "UPDATE store_purchases
                SET status = 'PAID', completed_at = now(), stripe_payment_intent_id = $2,
                    stripe_checkout_session_id = $3
              WHERE id = $1
              RETURNING *",
        )
        .bind(before.id)
        .bind(payment_intent)
        .bind(session_id)
        .fetch_one(&mut *tx)
        .await?;
        if before.status == PurchaseStatus::Cancelled && before.stock_reserved {
            sqlx::query(
                "UPDATE store_items SET remaining_stock = GREATEST(0, remaining_stock - 1)
                  WHERE id = $1 AND remaining_stock IS NOT NULL",
            )
            .bind(purchase.item_id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE store_items SET all_time_purchases = all_time_purchases + 1 WHERE id = $1")
            .bind(purchase.item_id)
            .execute(&mut *tx)
            .await?;
        let receipt: Receipt = sqlx::query_as(
            "INSERT INTO store_receipts
                (stripe_checkout_session_id, stripe_payment_intent_id, buyer_id, item_id, item_name,
                 store_id, server_id, store_name, quantity, list_price, discount_percentage, price,
                 currency, purchased_at)
             SELECT $1, $2, p.buyer_id, i.id, i.name, s.id, s.server_id,
                    COALESCE(s.override_name, sv.name), p.quantity, p.list_price,
                    p.discount_percentage, p.price, p.currency, p.completed_at
             FROM store_purchases p
             JOIN store_items i ON i.id = p.item_id
             JOIN item_stores s ON s.id = p.store_id
             JOIN servers sv ON sv.id = s.server_id
             WHERE p.id = $3
             RETURNING *",
        )
        .bind(session_id)
        .bind(payment_intent)
        .bind(purchase.id)
        .fetch_one(&mut *tx)
        .await?;
        let fulfillment: Fulfillment = sqlx::query_as(
            "INSERT INTO store_fulfillments (receipt_id, buyer_id, steam_id, item_id, server_id, rewards)
             SELECT $1, $2, $3, $4, s.server_id, $5 FROM item_stores s WHERE s.id = $6
             RETURNING *",
        )
        .bind(receipt.id)
        .bind(purchase.buyer_id)
        .bind(&purchase.steam_id)
        .bind(purchase.item_id)
        .bind(&purchase.rewards)
        .bind(purchase.store_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(MarkPaid::Paid(receipt, fulfillment))
    }
}

/// Cancels purchase `id` if it's pending and returns its stock. Part of a transaction.
async fn cancel_pending(conn: &mut PgConnection, id: i64) -> sqlx::Result<Option<Purchase>> {
    let purchase: Option<Purchase> = sqlx::query_as(
        "UPDATE store_purchases SET status = 'CANCELLED', completed_at = now()
          WHERE id = $1 AND status = 'PENDING'
          RETURNING *",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(p) = &purchase
        && p.stock_reserved
    {
        sqlx::query(
            "UPDATE store_items SET remaining_stock = remaining_stock + 1
              WHERE id = $1 AND remaining_stock IS NOT NULL",
        )
        .bind(p.item_id)
        .execute(conn)
        .await?;
    }
    Ok(purchase)
}

/// Receipt columns, the buyer as JSON and the fulfillment (if any) as JSON, from
/// `store_receipts r`.
macro_rules! select_receipt {
    ($rest:literal) => {
        concat!(
            "SELECT r.*,
                    json_build_object('id', u.id, 'username', u.username,
                                      'avatar_url', u.avatar_url) AS buyer,
                    CASE WHEN f.id IS NULL THEN NULL ELSE to_jsonb(f) END AS fulfillment
             FROM store_receipts r
             JOIN users u ON u.id = r.buyer_id
             LEFT JOIN store_fulfillments f ON f.receipt_id = r.id ",
            $rest
        )
    };
}

impl Receipt {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<ReceiptView>> {
        sqlx::query_as(select_receipt!("WHERE r.id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn find_by_payment_intent(db: &PgPool, payment_intent: &str) -> sqlx::Result<Option<Receipt>> {
        sqlx::query_as("SELECT * FROM store_receipts WHERE stripe_payment_intent_id = $1")
            .bind(payment_intent)
            .fetch_optional(db)
            .await
    }

    /// The buyer's receipts, newest first.
    pub async fn list_for_buyer(db: &PgPool, buyer_id: i64) -> sqlx::Result<Vec<ReceiptView>> {
        sqlx::query_as(select_receipt!("WHERE r.buyer_id = $1 ORDER BY r.purchased_at DESC, r.id DESC"))
            .bind(buyer_id)
            .fetch_all(db)
            .await
    }

    /// Receipts matching every filter, newest first.
    pub async fn list(db: &PgPool, filter: &ReceiptFilter) -> sqlx::Result<Vec<ReceiptView>> {
        let mut q: QueryBuilder<Postgres> = QueryBuilder::new(select_receipt!("WHERE true"));
        if let Some(status) = filter.status {
            q.push(" AND r.status = ").push_bind(status);
        }
        if let Some(store_id) = filter.store_id {
            q.push(" AND r.store_id = ").push_bind(store_id);
        }
        if let Some(buyer_id) = filter.buyer_id {
            q.push(" AND r.buyer_id = ").push_bind(buyer_id);
        }
        q.push(" ORDER BY r.purchased_at DESC, r.id DESC LIMIT ").push_bind(filter.limit);
        q.build_query_as().fetch_all(db).await
    }

    /// Settles the receipt as refunded or disputed and revokes its fulfillment. Only moves
    /// forward (PAID to DISPUTED or REFUNDED, DISPUTED to REFUNDED), so repeated or
    /// out-of-order webhooks are harmless. A refund also takes the purchase off the item's
    /// `all_time_purchases`. `None` if there is no such receipt or the move isn't allowed
    /// from its status; always `None` for `Paid`, which receipts never go back to.
    pub async fn set_status(db: &PgPool, id: i64, status: ReceiptStatus) -> sqlx::Result<Option<(Receipt, Option<Fulfillment>)>> {
        if status == ReceiptStatus::Paid {
            return Ok(None);
        }
        let mut tx = db.begin().await?;
        let receipt: Option<Receipt> = sqlx::query_as(
            "UPDATE store_receipts SET status = $2, status_changed_at = now()
              WHERE id = $1
                AND (status = 'PAID' OR (status = 'DISPUTED' AND $2 = 'REFUNDED'::receipt_status))
              RETURNING *",
        )
        .bind(id)
        .bind(status)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(receipt) = receipt else {
            return Ok(None);
        };
        if status == ReceiptStatus::Refunded
            && let Some(item_id) = receipt.item_id
        {
            sqlx::query(
                "UPDATE store_items SET all_time_purchases = GREATEST(0, all_time_purchases - $2)
                  WHERE id = $1",
            )
            .bind(item_id)
            .bind(receipt.quantity)
            .execute(&mut *tx)
            .await?;
        }
        let fulfillment = sqlx::query_as(
            "UPDATE store_fulfillments SET status = 'REVOKED', revoked_at = COALESCE(revoked_at, now())
              WHERE receipt_id = $1
              RETURNING *",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some((receipt, fulfillment)))
    }

    /// Receipts written in the past `days` days.
    pub async fn count_since_days(db: &PgPool, days: i32) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "SELECT count(*) FROM store_receipts WHERE purchased_at > now() - make_interval(days => $1)",
        )
        .bind(days)
        .fetch_one(db)
        .await
    }

    /// Revenue in the past `days` days, per currency, excluding refunded and disputed
    /// receipts.
    pub async fn income_since_days(db: &PgPool, days: i32) -> sqlx::Result<Vec<Income>> {
        sqlx::query_as(
            "SELECT currency, sum(price)::BIGINT AS amount FROM store_receipts
             WHERE status = 'PAID' AND purchased_at > now() - make_interval(days => $1)
             GROUP BY currency ORDER BY currency",
        )
        .bind(days)
        .fetch_all(db)
        .await
    }
}

impl Fulfillment {
    /// What the server's addon has to do: deliver pending (and retryable failed)
    /// fulfillments, and take back the rewards of revoked ones it had delivered. Oldest
    /// first; `steam_id` narrows it to one player.
    pub async fn work_for_server(db: &PgPool, server_id: i64, steam_id: Option<&str>) -> sqlx::Result<Vec<Fulfillment>> {
        sqlx::query_as(
            "SELECT * FROM store_fulfillments
             WHERE server_id = $1
               AND ($2::TEXT IS NULL OR steam_id = $2)
               AND ((status IN ('PENDING', 'FAILED') AND attempts < $3)
                    OR (status = 'REVOKED' AND fulfilled_at IS NOT NULL AND revocation_applied_at IS NULL))
             ORDER BY created_at, id",
        )
        .bind(server_id)
        .bind(steam_id)
        .bind(MAX_ATTEMPTS)
        .fetch_all(db)
        .await
    }

    /// Offers a failed fulfillment to the addon again with a fresh set of attempts, e.g.
    /// once whatever made it fail has been fixed. `None` unless it's failed and its receipt
    /// is still paid.
    pub async fn retry(db: &PgPool, id: i64) -> sqlx::Result<Option<Fulfillment>> {
        sqlx::query_as(
            "UPDATE store_fulfillments f SET status = 'PENDING', attempts = 0, last_error = NULL
              FROM store_receipts r
              WHERE f.id = $1 AND f.status = 'FAILED' AND r.id = f.receipt_id AND r.status = 'PAID'
              RETURNING f.*",
        )
        .bind(id)
        .fetch_optional(db)
        .await
    }

    pub async fn find_for_server(db: &PgPool, server_id: i64, id: i64) -> sqlx::Result<Option<Fulfillment>> {
        sqlx::query_as("SELECT * FROM store_fulfillments WHERE id = $1 AND server_id = $2")
            .bind(id)
            .bind(server_id)
            .fetch_optional(db)
            .await
    }

    /// Records the addon's report. A repeated `fulfilled` or `revoked` report changes
    /// nothing; each `failed` report counts as an attempt, up to [`MAX_ATTEMPTS`], after
    /// which further ones are ignored. Rewards delivered after a revocation are still
    /// recorded as delivered, so the revocation gets applied on the next sync. Returns the
    /// fulfillment as it stands and whether this report changed it; `None` if the server
    /// has no such fulfillment.
    pub async fn acknowledge(
        db: &PgPool,
        server_id: i64,
        id: i64,
        result: AckResult,
        error: Option<&str>,
    ) -> sqlx::Result<Option<(Fulfillment, bool)>> {
        let query = match result {
            AckResult::Fulfilled => sqlx::query(
                "UPDATE store_fulfillments
                    SET status = CASE WHEN status = 'REVOKED' THEN status ELSE 'FULFILLED' END,
                        fulfilled_at = COALESCE(fulfilled_at, now()),
                        attempts = attempts + 1, last_error = NULL
                  WHERE id = $1 AND server_id = $2
                    AND (status IN ('PENDING', 'FAILED') OR (status = 'REVOKED' AND fulfilled_at IS NULL))",
            )
            .bind(id)
            .bind(server_id),
            AckResult::Failed => sqlx::query(
                "UPDATE store_fulfillments
                    SET status = 'FAILED', attempts = attempts + 1, last_error = $3
                  WHERE id = $1 AND server_id = $2 AND status IN ('PENDING', 'FAILED') AND attempts < $4",
            )
            .bind(id)
            .bind(server_id)
            .bind(error)
            .bind(MAX_ATTEMPTS),
            AckResult::Revoked => sqlx::query(
                "UPDATE store_fulfillments
                    SET revocation_applied_at = COALESCE(revocation_applied_at, now())
                  WHERE id = $1 AND server_id = $2 AND status = 'REVOKED'",
            )
            .bind(id)
            .bind(server_id),
        };
        let changed = query.execute(db).await?.rows_affected() > 0;
        Ok(Self::find_for_server(db, server_id, id).await?.map(|f| (f, changed)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices() {
        assert_eq!(final_price(1000, 0), 1000);
        assert_eq!(final_price(1000, 25), 750);
        assert_eq!(final_price(999, 50), 500); // 499.5 rounds up
        assert_eq!(final_price(1000, 100), 0);
        assert_eq!(final_price(1000, 150), 0);
        assert_eq!(total_discount(30, 80), 100);
        assert_eq!(total_discount(10, 15), 25);
    }

    #[test]
    fn grantable() {
        let grantable = vec!["vip".to_owned()];
        let ok = vec![
            Reward::Usergroup { usergroup: "vip".into(), duration_seconds: None },
            Reward::Weapon { class: "weapon_crowbar".into() },
        ];
        assert_eq!(ungrantable_usergroup(&ok, &grantable), None);
        let mut bad = ok.clone();
        bad.push(Reward::Usergroup { usergroup: "superadmin".into(), duration_seconds: None });
        assert_eq!(ungrantable_usergroup(&bad, &grantable), Some("superadmin"));
        assert_eq!(ungrantable_usergroup(&ok, &[]), Some("vip"));
    }

    #[test]
    fn rewards_round_trip() {
        let rewards = vec![
            Reward::Usergroup { usergroup: "vip".into(), duration_seconds: Some(86_400) },
            Reward::Usergroup { usergroup: "donor".into(), duration_seconds: None },
            Reward::DarkrpMoney { amount: 5000 },
            Reward::Weapon { class: "weapon_crowbar".into() },
        ];
        let json = serde_json::to_string(&rewards).unwrap();
        assert!(json.contains(r#""type":"darkrp_money""#));
        assert!(!json.contains(r#""duration_seconds":null"#));
        let back: Vec<Reward> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rewards);
    }
}
