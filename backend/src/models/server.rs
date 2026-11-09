use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
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

/// The parts of a server admins edit on the website. Already validated (see
/// `routes::servers`). Gamemode, map and heartbeat come from the server itself.
pub struct ServerDetails {
    pub name: String,
    pub description: String,
    pub splash_url: Option<String>,
    pub icon_url: Option<String>,
    pub color: String,
    pub connection_address: String,
}

/// A freshly generated access key. The plaintext is shown to the admin once; only its
/// SHA-256 hash is stored (in `servers.access_key`), so a database leak can't link a server.
pub struct AccessKey {
    pub plaintext: String,
    pub hash: String,
}

impl AccessKey {
    const PREFIX: &str = "gnet_";

    /// `gnet_` followed by 32 random bytes in hex.
    pub fn generate() -> Result<Self, getrandom::Error> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes)?;
        let plaintext = format!("{}{}", Self::PREFIX, hex(&bytes));
        let hash = Self::hash(&plaintext);
        Ok(Self { plaintext, hash })
    }

    /// What's stored for `key`; compare against this when a server presents its key.
    pub fn hash(key: &str) -> String {
        hex(&Sha256::digest(key.trim().as_bytes()))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `$sql` followed by the server's public columns, for `RETURNING` / `SELECT`.
macro_rules! with_columns {
    ($sql:literal $(, $rest:literal)?) => {
        concat!(
            $sql,
            " id, name, description, gamemode, splash_url, icon_url, color,
              connection_address, map, last_heartbeat, linked, created_at"
            $(, $rest)?
        )
    };
}

impl Server {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Server>> {
        sqlx::query_as(with_columns!("SELECT", " FROM servers WHERE id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// Every server, oldest first.
    pub async fn list(db: &PgPool) -> sqlx::Result<Vec<Server>> {
        sqlx::query_as(with_columns!("SELECT", " FROM servers ORDER BY created_at, id"))
            .fetch_all(db)
            .await
    }

    /// Registers a server. It stays unlinked until the addon presents `access_key`.
    pub async fn create(
        db: &PgPool,
        details: &ServerDetails,
        access_key: &AccessKey,
    ) -> sqlx::Result<Server> {
        sqlx::query_as(with_columns!(
            "INSERT INTO servers
                 (name, description, splash_url, icon_url, color, connection_address, access_key)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING"
        ))
        .bind(&details.name)
        .bind(&details.description)
        .bind(&details.splash_url)
        .bind(&details.icon_url)
        .bind(&details.color)
        .bind(&details.connection_address)
        .bind(&access_key.hash)
        .fetch_one(db)
        .await
    }

    /// Replaces the server's editable details, or returns `None` if it doesn't exist.
    pub async fn update(
        db: &PgPool,
        id: i64,
        details: &ServerDetails,
    ) -> sqlx::Result<Option<Server>> {
        sqlx::query_as(with_columns!(
            "UPDATE servers
             SET name = $2, description = $3, splash_url = $4, icon_url = $5, color = $6,
                 connection_address = $7
             WHERE id = $1
             RETURNING"
        ))
        .bind(id)
        .bind(&details.name)
        .bind(&details.description)
        .bind(&details.splash_url)
        .bind(&details.icon_url)
        .bind(&details.color)
        .bind(&details.connection_address)
        .fetch_optional(db)
        .await
    }

    /// Replaces the access key and unlinks the server, so it must link again with the new
    /// key. Returns `None` if the server doesn't exist.
    pub async fn reset_access_key(
        db: &PgPool,
        id: i64,
        access_key: &AccessKey,
    ) -> sqlx::Result<Option<Server>> {
        sqlx::query_as(with_columns!(
            "UPDATE servers SET access_key = $2, linked = false WHERE id = $1 RETURNING"
        ))
        .bind(id)
        .bind(&access_key.hash)
        .fetch_optional(db)
        .await
    }

    /// Deletes the server and its player records. Returns whether it existed.
    pub async fn delete(db: &PgPool, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM servers WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_keys() {
        let a = AccessKey::generate().unwrap();
        let b = AccessKey::generate().unwrap();
        assert!(a.plaintext.starts_with("gnet_"));
        assert_eq!(a.plaintext.len(), 5 + 64);
        assert_ne!(a.plaintext, b.plaintext);
        assert_eq!(a.hash.len(), 64);
        assert_eq!(AccessKey::hash(&a.plaintext), a.hash);
        assert_ne!(a.hash, a.plaintext);
    }
}
