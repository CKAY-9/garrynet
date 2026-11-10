-- Notifications; see docs/NOTIFICATIONS.md. One row per recipient, created by the backend
-- when an event happens.

CREATE TYPE notification_type AS ENUM ('SUPPORT', 'STORE', 'FORUM', 'ACCOUNT', 'SYSTEM');

CREATE TABLE notifications (
    id         BIGSERIAL         PRIMARY KEY,
    -- The recipient.
    user_id    BIGINT            NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    type       notification_type NOT NULL,
    -- The kind of event that caused it, e.g. 'thread_reply'. Internal: it only decides
    -- which notifications collapse into one, and isn't sent to clients.
    event      TEXT              NOT NULL CHECK (btrim(event) <> ''),
    -- Plain text.
    title      TEXT              NOT NULL CHECK (btrim(title) <> '' AND char_length(title) <= 128),
    body       TEXT              CHECK (char_length(body) <= 512),
    -- A site-relative path, so a notification can't link off-site ('//host' and '/\host'
    -- would).
    url        TEXT              CHECK (url ~ '^/([^/\\]|$)' AND char_length(url) <= 512),
    created_at TIMESTAMPTZ       NOT NULL DEFAULT now(),
    -- NULL = unread.
    read_at    TIMESTAMPTZ
);

-- A user's list, newest first, and its cursor.
CREATE INDEX notifications_user_idx ON notifications (user_id, created_at DESC, id DESC);

-- The unread count behind the navigation badge.
CREATE INDEX notifications_unread_idx ON notifications (user_id) WHERE read_at IS NULL;

-- Collapsing repeats: a recipient has at most one unread notification per type, event and
-- url, which a repeat updates (INSERT ... ON CONFLICT). Notifications without a url have no
-- target to repeat, so they never collapse.
CREATE UNIQUE INDEX notifications_collapse_idx ON notifications (user_id, type, event, url)
    WHERE read_at IS NULL AND url IS NOT NULL;
