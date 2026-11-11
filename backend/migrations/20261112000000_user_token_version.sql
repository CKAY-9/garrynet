-- Revocable user sessions (docs/AUTH.md): user JWTs carry the version they were issued at
-- and are only accepted while it matches. Bumping it signs the user out everywhere.
-- Tokens issued before this migration have no version, which reads as 0, so they stay
-- valid until they expire.

ALTER TABLE users ADD COLUMN token_version BIGINT NOT NULL DEFAULT 0;
