-- A user's reputation is the likes minus dislikes across all of their forum posts (see
-- docs/USERS.md). Keep `users.reputation` in step with `forum_posts` here, so every way a
-- post changes counts: votes, deleting a reply, and deleting a thread, category or section
-- (whose posts go by cascade).

CREATE FUNCTION forum_posts_reputation() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    old_score INTEGER := 0;
    new_score INTEGER := 0;
BEGIN
    IF TG_OP <> 'INSERT' THEN
        old_score := cardinality(OLD.likes) - cardinality(OLD.dislikes);
    END IF;
    IF TG_OP <> 'DELETE' THEN
        new_score := cardinality(NEW.likes) - cardinality(NEW.dislikes);
    END IF;

    IF TG_OP = 'UPDATE' AND OLD.author_id = NEW.author_id THEN
        IF new_score <> old_score THEN
            UPDATE users SET reputation = reputation + new_score - old_score WHERE id = NEW.author_id;
        END IF;
        RETURN NULL;
    END IF;

    IF old_score <> 0 THEN
        UPDATE users SET reputation = reputation - old_score WHERE id = OLD.author_id;
    END IF;
    IF new_score <> 0 THEN
        UPDATE users SET reputation = reputation + new_score WHERE id = NEW.author_id;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER forum_posts_reputation
    AFTER INSERT OR DELETE OR UPDATE OF likes, dislikes, author_id ON forum_posts
    FOR EACH ROW EXECUTE FUNCTION forum_posts_reputation();

-- Bring existing totals in line.
UPDATE users u
   SET reputation = COALESCE(
       (SELECT sum(cardinality(p.likes) - cardinality(p.dislikes))
          FROM forum_posts p
         WHERE p.author_id = u.id), 0);
