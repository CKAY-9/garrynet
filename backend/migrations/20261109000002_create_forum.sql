-- The forum; see docs/FORUM.md. Sections hold categories, categories hold threads,
-- threads hold posts.

CREATE TABLE forum_sections (
    id           BIGSERIAL   PRIMARY KEY,
    name         TEXT        NOT NULL CHECK (btrim(name) <> ''),
    accent       TEXT        NOT NULL DEFAULT '#3b82f6'
                             CHECK (accent ~ '^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$'),
    icon_url     TEXT,
    description  TEXT        NOT NULL DEFAULT '' CHECK (char_length(description) <= 256),
    -- Listed highest to lowest; -1 is always last.
    position     INTEGER     NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_updated TIMESTAMPTZ
);

CREATE TABLE forum_categories (
    id           BIGSERIAL   PRIMARY KEY,
    name         TEXT        NOT NULL CHECK (btrim(name) <> ''),
    section_id   BIGINT      NOT NULL REFERENCES forum_sections (id) ON DELETE CASCADE,
    description  TEXT        NOT NULL DEFAULT '' CHECK (char_length(description) <= 256),
    -- Prevents new threads and posts.
    locked       BOOLEAN     NOT NULL DEFAULT false,
    -- Same ordering rules as sections.
    position     INTEGER     NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_updated TIMESTAMPTZ
);

CREATE INDEX forum_categories_section_id_idx ON forum_categories (section_id);

CREATE TABLE forum_threads (
    id             BIGSERIAL   PRIMARY KEY,
    author_id      BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    category_id    BIGINT      NOT NULL REFERENCES forum_categories (id) ON DELETE CASCADE,
    title          TEXT        NOT NULL CHECK (btrim(title) <> ''),
    -- The thread's first post. Created in the same transaction as the thread, so the
    -- foreign key (added below) is deferred.
    thread_post_id BIGINT      NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_activity  TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Prevents any changes to the thread.
    locked         BOOLEAN     NOT NULL DEFAULT false,
    -- Listed first, alongside other stickies.
    sticky         BOOLEAN     NOT NULL DEFAULT false,
    -- Users can copy this thread as a template for new threads.
    allow_template BOOLEAN     NOT NULL DEFAULT false
);

CREATE INDEX forum_threads_category_idx ON forum_threads (category_id, sticky DESC, last_activity DESC);

CREATE TABLE forum_posts (
    id         BIGSERIAL   PRIMARY KEY,
    author_id  BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    thread_id  BIGINT      NOT NULL REFERENCES forum_threads (id) ON DELETE CASCADE,
    content    TEXT        NOT NULL CHECK (btrim(content) <> ''),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    edited     TIMESTAMPTZ,
    -- Ids of users who liked / disliked the post.
    likes      BIGINT[]    NOT NULL DEFAULT '{}',
    dislikes   BIGINT[]    NOT NULL DEFAULT '{}'
);

CREATE INDEX forum_posts_thread_idx ON forum_posts (thread_id, created_at);
CREATE INDEX forum_posts_created_at_idx ON forum_posts (created_at);

ALTER TABLE forum_threads
    ADD CONSTRAINT forum_threads_thread_post_fk
    FOREIGN KEY (thread_post_id) REFERENCES forum_posts (id)
    ON DELETE CASCADE
    DEFERRABLE INITIALLY DEFERRED;
