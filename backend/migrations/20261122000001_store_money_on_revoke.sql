-- What a store does with DarkRP money it delivered when the purchase is refunded or disputed
-- (docs/STORE.md "Refunds and Disputes"): leave it with the player (the default), or take it
-- back as far as their balance allows, never below zero.
CREATE TYPE money_on_revoke AS ENUM ('KEEP', 'DEDUCT');

ALTER TABLE item_stores ADD COLUMN money_on_revoke money_on_revoke NOT NULL DEFAULT 'KEEP';

-- How much DarkRP money the addon took back when it revoked the fulfillment (NULL if none),
-- so reinstating the purchase delivers that much again.
ALTER TABLE store_fulfillments ADD COLUMN money_reclaimed BIGINT CHECK (money_reclaimed > 0);
