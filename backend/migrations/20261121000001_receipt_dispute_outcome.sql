-- How a disputed receipt's dispute ended, from Stripe's `charge.dispute.closed`: `won`,
-- `lost` or `warning_closed` (an inquiry closed without a chargeback). Staff can reinstate a
-- disputed receipt whose dispute was won, delivering its rewards again (docs/STORE.md).
ALTER TABLE store_receipts ADD COLUMN dispute_outcome TEXT;
