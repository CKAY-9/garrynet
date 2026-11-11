use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};

/// A player's record on one server (docs/USERS.md, "User Player"). The addon keeps it up
/// to date.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct UserPlayer {
    pub id: i64,
    pub server_id: i64,
    pub steam_id: String,
    /// GarryNet user id, if the player has an account.
    pub user_id: Option<i64>,
    pub ingame_name: String,
    /// Equals `first_joined` until the player returns.
    pub last_logon: DateTime<Utc>,
    pub first_joined: DateTime<Utc>,
    pub ingame_usergroup: String,
    /// Gamemode-specific data, e.g. `{"darkrp_money": 5000}`.
    pub stats: serde_json::Value,
    /// Minutes.
    pub time_played: i32,
}

impl UserPlayer {
    pub async fn find(
        db: &PgPool,
        server_id: i64,
        steam_id: &str,
    ) -> sqlx::Result<Option<UserPlayer>> {
        sqlx::query_as("SELECT * FROM user_players WHERE server_id = $1 AND steam_id = $2")
            .bind(server_id)
            .bind(steam_id)
            .fetch_optional(db)
            .await
    }

    /// Records players reported by the server's addon in a heartbeat. Creates the record on
    /// a first visit, links it to the player's GarryNet account if they have one, adds the
    /// reported playtime, and merges the gamemode stats: reported values replace the stored
    /// ones, counters are added to them (a stored value that isn't a number counts as 0).
    /// Stats the heartbeat doesn't mention are kept.
    pub async fn record_sessions(
        db: &mut PgConnection,
        server_id: i64,
        sessions: &[PlayerSession],
    ) -> sqlx::Result<()> {
        for session in sessions {
            sqlx::query(
                "INSERT INTO user_players
                     (server_id, steam_id, user_id, ingame_name, ingame_usergroup,
                      last_logon, first_joined, time_played, stats)
                 VALUES ($1, $2, (SELECT id FROM users WHERE steam_id = $2), $3, $4, $5, $5, $6,
                         $7::jsonb || $8::jsonb)
                 ON CONFLICT (server_id, steam_id) DO UPDATE SET
                     user_id = COALESCE(EXCLUDED.user_id, user_players.user_id),
                     ingame_name = EXCLUDED.ingame_name,
                     ingame_usergroup = EXCLUDED.ingame_usergroup,
                     last_logon = GREATEST(user_players.last_logon, EXCLUDED.last_logon),
                     time_played = user_players.time_played + EXCLUDED.time_played,
                     stats = user_players.stats || $7::jsonb || COALESCE((
                         SELECT jsonb_object_agg(
                             c.key,
                             c.value::numeric + CASE
                                 WHEN jsonb_typeof(user_players.stats -> c.key) = 'number'
                                 THEN (user_players.stats ->> c.key)::numeric
                                 ELSE 0
                             END)
                         FROM jsonb_each_text($8::jsonb) c
                     ), '{}'::jsonb)",
            )
            .bind(server_id)
            .bind(&session.steam_id)
            .bind(&session.name)
            .bind(&session.usergroup)
            .bind(session.connected_at)
            .bind(session.minutes)
            .bind(&session.stats)
            .bind(&session.counters)
            .execute(&mut *db)
            .await?;
        }
        Ok(())
    }
}

/// A player's current (or just ended) session on a server, as reported by the addon.
/// Already validated (see `routes::gnet`).
pub struct PlayerSession {
    /// SteamID64.
    pub steam_id: String,
    pub name: String,
    pub usergroup: String,
    /// When the session began; becomes `last_logon`.
    pub connected_at: DateTime<Utc>,
    /// Playtime not reported in an earlier heartbeat.
    pub minutes: i32,
    /// Latest gamemode stats (e.g. `darkrp_money`), replacing the stored values. Numbers,
    /// strings or booleans.
    pub stats: serde_json::Value,
    /// Gamemode counters not reported before (e.g. `sandbox_props_spawned`), added to the
    /// stored values. Positive integers.
    pub counters: serde_json::Value,
}
