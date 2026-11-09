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
    /// a first visit, links it to the player's GarryNet account if they have one, and adds
    /// the reported playtime.
    pub async fn record_sessions(
        db: &mut PgConnection,
        server_id: i64,
        sessions: &[PlayerSession],
    ) -> sqlx::Result<()> {
        for session in sessions {
            sqlx::query(
                "INSERT INTO user_players
                     (server_id, steam_id, user_id, ingame_name, ingame_usergroup,
                      last_logon, first_joined, time_played)
                 VALUES ($1, $2, (SELECT id FROM users WHERE steam_id = $2), $3, $4, $5, $5, $6)
                 ON CONFLICT (server_id, steam_id) DO UPDATE SET
                     user_id = COALESCE(EXCLUDED.user_id, user_players.user_id),
                     ingame_name = EXCLUDED.ingame_name,
                     ingame_usergroup = EXCLUDED.ingame_usergroup,
                     last_logon = GREATEST(user_players.last_logon, EXCLUDED.last_logon),
                     time_played = user_players.time_played + EXCLUDED.time_played",
            )
            .bind(server_id)
            .bind(&session.steam_id)
            .bind(&session.name)
            .bind(&session.usergroup)
            .bind(session.connected_at)
            .bind(session.minutes)
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
}
