-- Rotating a server's JWT (docs/ADDON.md#server-tokens) bumps `token_version`, revoking
-- the token it replaces. The replaced token is still accepted until this time, so requests
-- already in flight succeed and a server whose heartbeat response (carrying the new token)
-- got lost can collect it again. NULL when nothing was rotated, e.g. after linking.
ALTER TABLE servers ADD COLUMN previous_token_valid_until TIMESTAMPTZ;
