-- Attaching files to forum posts and support messages; see docs/STORAGE.md. A file is
-- attached to at most one thing, by its author, when the post or message is created.

ALTER TABLE files
    ADD COLUMN forum_post_id      BIGINT REFERENCES forum_posts (id) ON DELETE SET NULL,
    ADD COLUMN support_message_id BIGINT REFERENCES support_messages (id) ON DELETE SET NULL,
    ADD CONSTRAINT files_one_attachment CHECK (forum_post_id IS NULL OR support_message_id IS NULL);

CREATE INDEX files_forum_post_idx ON files (forum_post_id) WHERE forum_post_id IS NOT NULL;
CREATE INDEX files_support_message_idx ON files (support_message_id) WHERE support_message_id IS NOT NULL;
