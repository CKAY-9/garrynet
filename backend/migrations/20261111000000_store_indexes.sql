-- Indexes for the store's foreign keys that had none (see docs/STORE.md). Without them,
-- deleting a server or store scans these tables to cascade or null out its rows, and the
-- finance receipt list's store filter scans every receipt.

CREATE INDEX store_purchases_store_idx ON store_purchases (store_id);
CREATE INDEX store_purchases_item_idx ON store_purchases (item_id);
CREATE INDEX store_receipts_store_idx ON store_receipts (store_id, purchased_at DESC);
CREATE INDEX store_receipts_server_idx ON store_receipts (server_id);
CREATE INDEX store_receipts_item_idx ON store_receipts (item_id);
CREATE INDEX store_fulfillments_item_idx ON store_fulfillments (item_id);
