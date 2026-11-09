-- User search counts and sorts by each user's posts.
CREATE INDEX forum_posts_author_id_idx ON forum_posts (author_id);
