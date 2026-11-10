-- The store; see docs/STORE.md. Each server can have one item store whose items grant
-- in-game rewards. Purchases go through Stripe Checkout; receipts and fulfillments are
-- kept after the items (or the server) they came from are gone.

CREATE TYPE purchase_status AS ENUM ('PENDING', 'PAID', 'CANCELLED');
CREATE TYPE receipt_status AS ENUM ('PAID', 'REFUNDED', 'DISPUTED');
CREATE TYPE fulfillment_status AS ENUM ('PENDING', 'FULFILLED', 'FAILED', 'REVOKED');

CREATE TABLE item_stores (
    id                   BIGSERIAL   PRIMARY KEY,
    -- One store per server.
    server_id            BIGINT      NOT NULL UNIQUE REFERENCES servers (id) ON DELETE CASCADE,
    -- Hidden from /store and nothing can be bought while false.
    enabled              BOOLEAN     NOT NULL DEFAULT true,
    -- ISO 4217, lowercase. Applies to every item in the store.
    currency             TEXT        NOT NULL DEFAULT 'usd' CHECK (currency ~ '^[a-z]{3}$'),
    -- The store is named after its server unless this is set.
    override_name        TEXT        CHECK (override_name IS NULL OR btrim(override_name) <> ''),
    -- Shown at the top of the store page.
    store_message        TEXT,
    -- Garry's Mod usergroups items may grant. Only Elevated Control can change it.
    grantable_usergroups TEXT[]      NOT NULL DEFAULT '{}',
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_updated         TIMESTAMPTZ
);

CREATE TABLE store_discounts (
    id           BIGSERIAL   PRIMARY KEY,
    store_id     BIGINT      NOT NULL REFERENCES item_stores (id) ON DELETE CASCADE,
    -- Percent off, added to each item's own discount.
    discount     INTEGER     NOT NULL CHECK (discount BETWEEN 0 AND 100),
    message      TEXT        NOT NULL DEFAULT '',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_updated TIMESTAMPTZ,
    -- Active while now() is within [starts_at, ends_at); either end may be open.
    starts_at    TIMESTAMPTZ,
    ends_at      TIMESTAMPTZ,
    CHECK (starts_at IS NULL OR ends_at IS NULL OR ends_at > starts_at)
);

CREATE INDEX store_discounts_store_idx ON store_discounts (store_id);

CREATE TABLE store_items (
    id                 BIGSERIAL   PRIMARY KEY,
    store_id           BIGINT      NOT NULL REFERENCES item_stores (id) ON DELETE CASCADE,
    name               TEXT        NOT NULL CHECK (btrim(name) <> ''),
    -- Plain text.
    short_description  TEXT        NOT NULL DEFAULT '',
    -- Markdown.
    full_description   TEXT        NOT NULL DEFAULT '',
    image_url          TEXT,
    -- Ascending.
    sort_order         INTEGER     NOT NULL DEFAULT 0,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_updated       TIMESTAMPTZ,
    -- Listed but not buyable while false.
    available          BOOLEAN     NOT NULL DEFAULT true,
    -- Soft delete: hidden everywhere, kept for receipts and fulfillments.
    archived           BOOLEAN     NOT NULL DEFAULT false,
    -- Minor units of the store's currency (e.g. cents).
    price              BIGINT      NOT NULL CHECK (price >= 0),
    -- Percent off, before store-wide discounts.
    discount           INTEGER     NOT NULL DEFAULT 0 CHECK (discount BETWEEN 0 AND 100),
    -- Limited releases. NULL means unlimited.
    starting_stock     INTEGER     CHECK (starting_stock IS NULL OR starting_stock >= 0),
    -- Excludes stock reserved by pending purchases.
    remaining_stock    INTEGER     CHECK (remaining_stock IS NULL OR remaining_stock >= 0),
    -- How many times one user may buy it (paid or pending). NULL means no limit.
    max_per_user       INTEGER     CHECK (max_per_user IS NULL OR max_per_user > 0),
    -- Incremented when a purchase is paid.
    all_time_purchases INTEGER     NOT NULL DEFAULT 0,
    -- Array of { type: "usergroup" | "darkrp_money" | "weapon", ... }.
    rewards            JSONB       NOT NULL DEFAULT '[]'
);

CREATE INDEX store_items_store_idx ON store_items (store_id, archived, sort_order, id);

-- A checkout attempt. PENDING while the Stripe Checkout Session is open; PAID once the
-- `checkout.session.completed` webhook arrives; CANCELLED if the session expired or the
-- buyer backed out. Pending purchases hold a unit of limited stock.
CREATE TABLE store_purchases (
    id                         BIGSERIAL       PRIMARY KEY,
    store_id                   BIGINT          NOT NULL REFERENCES item_stores (id) ON DELETE CASCADE,
    item_id                    BIGINT          NOT NULL REFERENCES store_items (id) ON DELETE CASCADE,
    buyer_id                   BIGINT          NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- The buyer's SteamID64 at purchase time: who the rewards go to in-game.
    steam_id                   TEXT            NOT NULL,
    status                     purchase_status NOT NULL DEFAULT 'PENDING',
    -- Set once the session is created (right after the row).
    stripe_checkout_session_id TEXT            UNIQUE,
    stripe_payment_intent_id   TEXT,
    quantity                   INTEGER         NOT NULL DEFAULT 1 CHECK (quantity > 0),
    -- Minor units. What the item listed at, the total percentage off, and what's charged.
    list_price                 BIGINT          NOT NULL,
    discount_percentage        INTEGER         NOT NULL,
    price                      BIGINT          NOT NULL,
    currency                   TEXT            NOT NULL,
    -- The item's rewards when the buyer checked out; copied to the fulfillment.
    rewards                    JSONB           NOT NULL,
    -- Whether a unit of limited stock was taken for this purchase (returned on cancel).
    stock_reserved             BOOLEAN         NOT NULL DEFAULT false,
    created_at                 TIMESTAMPTZ     NOT NULL DEFAULT now(),
    completed_at               TIMESTAMPTZ
);

CREATE INDEX store_purchases_buyer_item_idx ON store_purchases (buyer_id, item_id, status);
CREATE INDEX store_purchases_pending_idx ON store_purchases (created_at) WHERE status = 'PENDING';

-- One per paid purchase. The Stripe ids are unique, which makes webhook handling idempotent.
-- The item and server are kept by name as well, so a receipt outlives both.
CREATE TABLE store_receipts (
    id                         BIGSERIAL      PRIMARY KEY,
    stripe_checkout_session_id TEXT           NOT NULL UNIQUE,
    stripe_payment_intent_id   TEXT           NOT NULL UNIQUE,
    buyer_id                   BIGINT         NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    item_id                    BIGINT         REFERENCES store_items (id) ON DELETE SET NULL,
    item_name                  TEXT           NOT NULL,
    store_id                   BIGINT         REFERENCES item_stores (id) ON DELETE SET NULL,
    server_id                  BIGINT         REFERENCES servers (id) ON DELETE SET NULL,
    store_name                 TEXT           NOT NULL,
    quantity                   INTEGER        NOT NULL DEFAULT 1,
    status                     receipt_status NOT NULL DEFAULT 'PAID',
    purchased_at               TIMESTAMPTZ    NOT NULL DEFAULT now(),
    list_price                 BIGINT         NOT NULL,
    discount_percentage        INTEGER        NOT NULL,
    price                      BIGINT         NOT NULL,
    currency                   TEXT           NOT NULL,
    -- Set when the receipt stops being PAID.
    status_changed_at          TIMESTAMPTZ
);

CREATE INDEX store_receipts_buyer_idx ON store_receipts (buyer_id, purchased_at DESC);
CREATE INDEX store_receipts_purchased_idx ON store_receipts (purchased_at DESC);

-- Delivery of a receipt's rewards in-game, done by the server's addon.
CREATE TABLE store_fulfillments (
    id                    BIGSERIAL          PRIMARY KEY,
    receipt_id            BIGINT             NOT NULL UNIQUE REFERENCES store_receipts (id) ON DELETE CASCADE,
    buyer_id              BIGINT             NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    steam_id              TEXT               NOT NULL,
    item_id               BIGINT             REFERENCES store_items (id) ON DELETE SET NULL,
    -- Nothing to deliver once the server is gone.
    server_id             BIGINT             NOT NULL REFERENCES servers (id) ON DELETE CASCADE,
    -- Snapshot from the purchase; later item edits don't change it.
    rewards               JSONB              NOT NULL,
    status                fulfillment_status NOT NULL DEFAULT 'PENDING',
    attempts              INTEGER            NOT NULL DEFAULT 0,
    last_error            TEXT,
    created_at            TIMESTAMPTZ        NOT NULL DEFAULT now(),
    fulfilled_at          TIMESTAMPTZ,
    revoked_at            TIMESTAMPTZ,
    -- When the addon confirmed it took the rewards back after a revocation.
    revocation_applied_at TIMESTAMPTZ
);

CREATE INDEX store_fulfillments_server_idx ON store_fulfillments (server_id, status);
CREATE INDEX store_fulfillments_buyer_idx ON store_fulfillments (buyer_id);

-- The default `admin` group has every permission, but existing groups are never
-- overwritten on startup, so grant Store Management (0x2000) and Store Finance (0x4000).
UPDATE usergroups SET permissions = permissions | 24576, last_updated = now()
WHERE lower(name) = 'admin' AND permissions & 24576 <> 24576;
