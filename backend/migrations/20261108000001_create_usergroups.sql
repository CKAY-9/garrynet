CREATE TYPE usergroup_effect AS ENUM ('halo', 'rainbow', 'flash', 'none');

CREATE TABLE usergroups (
    id           BIGSERIAL        PRIMARY KEY,
    name         TEXT             NOT NULL CHECK (btrim(name) <> ''),
    effect       usergroup_effect NOT NULL DEFAULT 'none',
    color        TEXT             NOT NULL DEFAULT '#ffffff'
                                  CHECK (color ~ '^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$'),
    -- Permission bit flags; see `Permissions` in src/models/permissions.rs.
    permissions  BIGINT           NOT NULL DEFAULT 0,
    icon_url     TEXT,
    created_at   TIMESTAMPTZ      NOT NULL DEFAULT now(),
    last_updated TIMESTAMPTZ
);

-- Group names are unique regardless of case, so `usergroup add 1 Admin` is unambiguous.
CREATE UNIQUE INDEX usergroups_name_key ON usergroups (lower(name));

-- The design's `usergroup.members` list, stored relationally so deleting a user or group
-- cleans up its memberships.
CREATE TABLE usergroup_members (
    usergroup_id BIGINT      NOT NULL REFERENCES usergroups (id) ON DELETE CASCADE,
    user_id      BIGINT      NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    added_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (usergroup_id, user_id)
);

CREATE INDEX usergroup_members_user_id_idx ON usergroup_members (user_id);
