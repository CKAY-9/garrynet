-- Profile settings users manage themselves at /settings (docs/USERS.md "Profile Settings").

-- Hides the user's bio, reputation, threads and posts from everyone but themselves and staff.
ALTER TABLE users ADD COLUMN private BOOLEAN NOT NULL DEFAULT false;

-- Bios were never editable before, so every existing one is empty.
ALTER TABLE users ADD CONSTRAINT users_bio_length CHECK (char_length(bio) <= 1000);

-- A user's threads, listed on their profile. Their posts use forum_posts_author_id_idx.
CREATE INDEX forum_threads_author_idx ON forum_threads (author_id);
