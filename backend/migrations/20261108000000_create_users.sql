CREATE TYPE user_status AS ENUM ('ONLINE', 'IDLE', 'OFFLINE');

CREATE TABLE users (
    id         BIGSERIAL   PRIMARY KEY,
    username   TEXT        NOT NULL,
    email      TEXT,
    avatar_url TEXT        NOT NULL,
    steam_id   TEXT        NOT NULL UNIQUE,
    bio        TEXT        NOT NULL DEFAULT '',
    reputation INTEGER     NOT NULL DEFAULT 0,
    status     user_status NOT NULL DEFAULT 'OFFLINE',
    joined     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen  TIMESTAMPTZ NOT NULL DEFAULT now()
);
