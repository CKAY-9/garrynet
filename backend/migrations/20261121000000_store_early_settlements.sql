-- Refunds and disputes Stripe reported for a payment GravNet hasn't recorded yet: Stripe
-- doesn't guarantee the order of webhook events, and the payment's own event may be failing
-- and waiting to be retried. Applied to the receipt when the payment is recorded (see
-- `Purchase::mark_paid`). Rows for payments that never turn up (other integrations on the
-- Stripe account) are swept after a while.
CREATE TABLE store_early_settlements (
    stripe_payment_intent_id TEXT           PRIMARY KEY,
    -- A refund outranks a dispute, as on receipts.
    status                   receipt_status NOT NULL CHECK (status <> 'PAID'),
    created_at               TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX store_early_settlements_created_idx ON store_early_settlements (created_at);
