-- The community this deployment serves. Holds at most one row; the backend creates a
-- skeleton row on startup if there is none.
CREATE TABLE community (
    id                BIGSERIAL   PRIMARY KEY,
    name              TEXT        NOT NULL CHECK (btrim(name) <> ''),
    description       TEXT        NOT NULL DEFAULT '',
    logo_url          TEXT,
    -- Image URL shown behind the homepage header.
    splash_background TEXT,
    external_links    TEXT[]      NOT NULL DEFAULT '{}',
    -- Array of { user_id?, name, profile_picture, role, description }.
    key_figures       JSONB       NOT NULL DEFAULT '[]' CHECK (jsonb_typeof(key_figures) = 'array'),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_updated      TIMESTAMPTZ
);

-- One community per deployment.
CREATE UNIQUE INDEX community_singleton ON community ((true));
