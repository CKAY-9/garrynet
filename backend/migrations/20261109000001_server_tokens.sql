-- Linking servers (docs/ADDON.md). The access key is single-use: linking clears it, and the
-- addon authenticates with a server JWT from then on.
ALTER TABLE servers ALTER COLUMN access_key DROP NOT NULL;

-- Server JWTs carry the version they were issued for and are only accepted while it matches.
-- Bumped when the server links, unlinks or is given a new access key, which revokes every
-- token issued before.
ALTER TABLE servers ADD COLUMN token_version BIGINT NOT NULL DEFAULT 0;
