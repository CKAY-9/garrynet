-- Rate limits count what a user wrote recently (docs/FORUM.md, docs/SUPPORT.md).
CREATE INDEX forum_threads_author_created_idx ON forum_threads (author_id, created_at);
CREATE INDEX support_messages_author_created_idx ON support_messages (author_id, created_at);
