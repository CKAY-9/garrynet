use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

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
}
