-- How many forum posts (threads' first posts and replies) each user has written, so user
-- search can show and sort by it without counting `forum_posts` for every user. Kept in
-- step with `forum_posts` here, like `users.reputation` (see the `forum_reputation`
-- migration), so deleting a reply, thread, category or section counts too.

ALTER TABLE users ADD COLUMN post_count INTEGER NOT NULL DEFAULT 0;

CREATE FUNCTION forum_posts_count() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.author_id = NEW.author_id THEN
        RETURN NULL;
    END IF;
    IF TG_OP <> 'INSERT' THEN
        UPDATE users SET post_count = post_count - 1 WHERE id = OLD.author_id;
    END IF;
    IF TG_OP <> 'DELETE' THEN
        UPDATE users SET post_count = post_count + 1 WHERE id = NEW.author_id;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER forum_posts_count
    AFTER INSERT OR DELETE OR UPDATE OF author_id ON forum_posts
    FOR EACH ROW EXECUTE FUNCTION forum_posts_count();

-- Bring existing totals in line.
UPDATE users u
   SET post_count = (SELECT count(*) FROM forum_posts p WHERE p.author_id = u.id);

-- User search's sort orders (`User::search`). Private users sort last. `last_seen` is left
-- unindexed so the once-a-minute `touch` stays a HOT update.
CREATE INDEX users_post_count_idx ON users (private, post_count DESC, id);
CREATE INDEX users_reputation_desc_idx ON users (private, reputation DESC, id);
CREATE INDEX users_reputation_asc_idx ON users (private, reputation, id);
