-- Garry's Mod servers; see docs/SERVERS.md. Registered on the website, then linked by
-- pasting `access_key` into the GarryNet addon.
CREATE TABLE servers (
    id                 BIGSERIAL   PRIMARY KEY,
    name               TEXT        NOT NULL CHECK (btrim(name) <> ''),
    description        TEXT        NOT NULL DEFAULT '',
    -- Empty until the server's first heartbeat.
    gamemode           TEXT        NOT NULL DEFAULT '',
    splash_url         TEXT,
    icon_url           TEXT,
    color              TEXT        NOT NULL DEFAULT '#3b82f6'
                                   CHECK (color ~ '^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$'),
    connection_address TEXT        NOT NULL DEFAULT '',
    -- Empty until the server's first heartbeat.
    map                TEXT        NOT NULL DEFAULT '',
    access_key         TEXT        NOT NULL UNIQUE,
    -- Null until the server's first heartbeat.
    last_heartbeat     TIMESTAMPTZ,
    linked             BOOLEAN     NOT NULL DEFAULT false,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Per-server player records (docs/USERS.md, "User Player"), kept up to date by the addon.
CREATE TABLE user_players (
    id               BIGSERIAL   PRIMARY KEY,
    server_id        BIGINT      NOT NULL REFERENCES servers (id) ON DELETE CASCADE,
    steam_id         TEXT        NOT NULL,
    -- The player's GarryNet account, if they have one.
    user_id          BIGINT      REFERENCES users (id) ON DELETE SET NULL,
    ingame_name      TEXT        NOT NULL,
    last_logon       TIMESTAMPTZ NOT NULL DEFAULT now(),
    first_joined     TIMESTAMPTZ NOT NULL DEFAULT now(),
    ingame_usergroup TEXT        NOT NULL DEFAULT 'user',
    -- Gamemode-specific data, e.g. `{"darkrp_money": 5000}`.
    stats            JSONB       NOT NULL DEFAULT '{}',
    -- Minutes; the addon adds one every minute.
    time_played      INTEGER     NOT NULL DEFAULT 0 CHECK (time_played >= 0),
    UNIQUE (server_id, steam_id)
);

CREATE INDEX user_players_user_id_idx ON user_players (user_id);
