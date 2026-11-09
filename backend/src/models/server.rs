use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

/// A Garry's Mod server, minus its `access_key`, so it's safe to send to anyone.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Server {
    pub id: i64,
    pub name: String,
    pub description: String,
    /// Empty until the first heartbeat.
    pub gamemode: String,
    /// Shown on the website, not on the loading screen.
    pub splash_url: Option<String>,
    pub icon_url: Option<String>,
    /// Hex color, e.g. `#3b82f6`.
    pub color: String,
    pub connection_address: String,
    /// Empty until the first heartbeat.
    pub map: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub linked: bool,
    pub created_at: DateTime<Utc>,
}

impl Server {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Server>> {
        sqlx::query_as(
            "SELECT id, name, description, gamemode, splash_url, icon_url, color,
                    connection_address, map, last_heartbeat, linked, created_at
             FROM servers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(db)
        .await
    }
}
