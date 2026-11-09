CREATE TYPE loading_screen_style AS ENUM ('aero', 'sleek');

-- The community's loading screen settings, shared by every server. Always exactly one row.
CREATE TABLE loading_screen_config (
    id    SMALLINT             PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    style loading_screen_style NOT NULL DEFAULT 'aero'
);

INSERT INTO loading_screen_config DEFAULT VALUES;
