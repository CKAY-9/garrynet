-- Private messages between two users; see docs/PRIVATE_MESSAGES.md. There is no chat
-- table: a chat is the pair of users, addressed by the other user's id. Deleting a chat
-- and temporary messages only hide rows (per side, or after 24 hours); staff still see
-- everything.

CREATE TABLE private_messages (
    id                   BIGSERIAL   PRIMARY KEY,
    author_id            BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    recipient_id         BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Markdown, sanitized by the backend.
    content              TEXT        NOT NULL CHECK (btrim(content) <> ''),
    -- Set by the sender: hidden from both users 24 hours after created_at.
    temporary            BOOLEAN     NOT NULL DEFAULT false,
    -- Each user deleting the chat only hides it from themselves.
    deleted_by_author    BOOLEAN     NOT NULL DEFAULT false,
    deleted_by_recipient BOOLEAN     NOT NULL DEFAULT false,
    -- NULL = the recipient hasn't opened the chat since it arrived.
    read_at              TIMESTAMPTZ,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (author_id <> recipient_id)
);

-- A chat's messages from either side, newest first, and its cursor; also the chat list.
CREATE INDEX private_messages_author_idx
    ON private_messages (author_id, recipient_id, created_at DESC, id DESC);
CREATE INDEX private_messages_recipient_idx
    ON private_messages (recipient_id, author_id, created_at DESC, id DESC);
-- The unread count behind the navigation badge.
CREATE INDEX private_messages_unread_idx ON private_messages (recipient_id) WHERE read_at IS NULL;
-- The send rate limit: a sender's recent messages to anyone.
CREATE INDEX private_messages_sent_idx ON private_messages (author_id, created_at DESC);

-- Files attached to a private message are only served to its author, its recipient and
-- staff. A file is still attached to at most one thing.
ALTER TABLE files
    ADD COLUMN private_message_id BIGINT REFERENCES private_messages (id) ON DELETE SET NULL,
    DROP CONSTRAINT files_one_attachment,
    ADD CONSTRAINT files_one_attachment
        CHECK (num_nonnulls(forum_post_id, support_message_id, private_message_id) <= 1);

CREATE INDEX files_private_message_idx ON files (private_message_id) WHERE private_message_id IS NOT NULL;
