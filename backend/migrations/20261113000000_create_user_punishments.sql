-- Website punishments (docs/USERS.md "User Punishment"). A ban takes away every permission
-- (support only); a suspension takes away posting and messaging. Lifted and expired
-- punishments are kept for history.

CREATE TYPE punishment_kind AS ENUM ('BAN', 'SUSPEND');

CREATE TABLE user_punishments (
    id          BIGSERIAL       PRIMARY KEY,
    user_id     BIGINT          NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind        punishment_kind NOT NULL,
    reason      TEXT            NOT NULL CHECK (btrim(reason) <> ''),
    -- Null once the issuer's account is gone.
    issuer_id   BIGINT          REFERENCES users (id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ     NOT NULL DEFAULT now(),
    -- When it stops being enforced; null for permanent. Suspensions always end.
    ends        TIMESTAMPTZ     CHECK (ends IS NULL OR ends > created_at),
    -- False once lifted early, or once an expired one is replaced by a new one. An expired
    -- punishment can stay true; it just isn't enforced any more.
    active      BOOLEAN         NOT NULL DEFAULT true,
    -- Who lifted it early, when and why. Null lifted_by with a lifted_at: the backend
    -- shell, or an account since deleted.
    lifted_by   BIGINT          REFERENCES users (id) ON DELETE SET NULL,
    lifted_at   TIMESTAMPTZ,
    lift_reason TEXT,
    CHECK (kind <> 'SUSPEND' OR ends IS NOT NULL),
    CHECK (lifted_at IS NULL OR NOT active)
);

-- At most one of each kind is active per user. Issuing a new one first deactivates an
-- expired one.
CREATE UNIQUE INDEX user_punishments_one_active ON user_punishments (user_id, kind) WHERE active;
-- A user's history, newest first.
CREATE INDEX user_punishments_user_idx ON user_punishments (user_id, created_at DESC);

-- Whether the addon keeps users banned on GarryNet off the community's game servers
-- (docs/ADDON.md "Ban Enforcement").
ALTER TABLE community ADD COLUMN ban_from_servers BOOLEAN NOT NULL DEFAULT false;
