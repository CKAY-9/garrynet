-- How much of each store payment Stripe has refunded, and how its latest refund stands
-- (docs/STORE.md "Refunds and Disputes"). Replaced with Stripe's current figures whenever it
-- reports a refund or a change to one, including refunds made from the Stripe dashboard,
-- partial ones, and ones that fail after being accepted. Keyed by PaymentIntent rather than
-- receipt, since a refund can be reported before its payment is recorded; receipts join it
-- on `stripe_payment_intent_id`. Rows for payments that never get a receipt are swept.
CREATE TABLE store_payment_refunds (
    stripe_payment_intent_id TEXT        PRIMARY KEY,
    -- Minor units: what Stripe captured, tax included.
    amount_charged           BIGINT      NOT NULL CHECK (amount_charged >= 0),
    -- Minor units: refunds that succeeded or are still under way. Failed and cancelled
    -- refunds don't count.
    amount_refunded          BIGINT      NOT NULL CHECK (amount_refunded >= 0),
    -- Stripe's status of the most recent refund (`pending`, `requires_action`, `succeeded`,
    -- `failed` or `canceled`), and why it failed, if it did.
    latest_status            TEXT,
    latest_failure_reason    TEXT,
    -- Refunds that failed or were cancelled; names the next refund's idempotency key.
    failed_refunds           INT         NOT NULL DEFAULT 0 CHECK (failed_refunds >= 0),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX store_payment_refunds_updated_idx ON store_payment_refunds (updated_at);
