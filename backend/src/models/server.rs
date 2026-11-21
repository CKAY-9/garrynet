use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool};

use super::store::Receipt;

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

    /// Replaces the access key and unlinks the server, revoking its tokens, so it must link
    /// again with the new key. Returns `None` if the server doesn't exist.
    pub async fn reset_access_key(
        db: &PgPool,
        id: i64,
        access_key: &AccessKey,
    ) -> sqlx::Result<Option<Server>> {
        sqlx::query_as(with_columns!(
            "UPDATE servers
             SET access_key = $2, linked = false, token_version = token_version + 1,
                 previous_token_valid_until = NULL
             WHERE id = $1
             RETURNING"
        ))
        .bind(id)
        .bind(&access_key.hash)
        .fetch_optional(db)
        .await
    }

    /// Links the server whose access key is `key`: marks it linked, revokes the key (it's
    /// single-use) and any earlier tokens. Returns the server and the token version to issue
    /// its JWT for, or `None` if no server has that key.
    pub async fn link(db: &PgPool, key: &str) -> sqlx::Result<Option<(Server, i64)>> {
        let row: Option<(i64, i64)> = sqlx::query_as(
            "UPDATE servers
             SET access_key = NULL, linked = true, token_version = token_version + 1,
                 previous_token_valid_until = NULL
             WHERE access_key = $1
             RETURNING id, token_version",
        )
        .bind(AccessKey::hash(key))
        .fetch_optional(db)
        .await?;
        let Some((id, version)) = row else {
            return Ok(None);
        };
        Ok(Server::find_by_id(db, id).await?.map(|server| (server, version)))
    }

    /// Unlinks the server at its own request: revokes its tokens and any access key. An
    /// admin has to generate a new key before it can link again.
    pub async fn unlink(db: &PgPool, id: i64) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE servers
             SET access_key = NULL, linked = false, token_version = token_version + 1,
                 previous_token_valid_until = NULL
             WHERE id = $1",
        )
        .bind(id)
        .execute(db)
        .await?;
        Ok(())
    }

    /// The server's current token version, if a token of version `version` is accepted:
    /// the current version, or the one rotation just replaced while it is still within its
    /// grace period. `None` if the token is revoked, or the server doesn't exist or isn't
    /// linked.
    pub async fn accepted_token_version(db: &PgPool, id: i64, version: i64) -> sqlx::Result<Option<i64>> {
        sqlx::query_scalar(
            "SELECT token_version FROM servers
             WHERE id = $1 AND linked
               AND (token_version = $2
                    OR (token_version = $2 + 1 AND previous_token_valid_until > now()))",
        )
        .bind(id)
        .bind(version)
        .fetch_optional(db)
        .await
    }

    /// Rotates the server's token: bumps its version from `from`, revoking tokens of that
    /// version once `grace` has passed. Returns the version to issue the new token for, or
    /// `None` if `from` is no longer current (another request rotated it first) or the
    /// server is gone or unlinked.
    pub async fn rotate_token(db: &PgPool, id: i64, from: i64, grace: chrono::Duration) -> sqlx::Result<Option<i64>> {
        sqlx::query_scalar(
            "UPDATE servers
             SET token_version = token_version + 1,
                 previous_token_valid_until = now() + make_interval(secs => $3)
             WHERE id = $1 AND linked AND token_version = $2
             RETURNING token_version",
        )
        .bind(id)
        .bind(from)
        .bind(grace.num_seconds() as f64)
        .fetch_optional(db)
        .await
    }

    /// Records a heartbeat: the server's current gamemode and map, checked in now.
    pub async fn heartbeat(
        db: &mut PgConnection,
        id: i64,
        gamemode: &str,
        map: &str,
    ) -> sqlx::Result<Option<Server>> {
        sqlx::query_as(with_columns!(
            "UPDATE servers SET gamemode = $2, map = $3, last_heartbeat = now()
             WHERE id = $1
             RETURNING"
        ))
        .bind(id)
        .bind(gamemode)
        .bind(map)
        .fetch_optional(db)
        .await
    }

    /// Deletes the server with its player records, store, purchases and fulfillments, unless
    /// paid purchases from its store haven't been delivered yet (see
    /// [`Receipt::count_undelivered`]): deleting would drop them unseen. The server's row is
    /// locked for the check, and recording a payment writes rows referencing it, so a payment
    /// recorded meanwhile is either counted or fails for want of its server (the webhook then
    /// logs it for staff to refund).
    pub async fn delete(db: &PgPool, id: i64) -> sqlx::Result<Delete> {
        let mut tx = db.begin().await?;
        let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM servers WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Ok(Delete::NotFound);
        }
        let undelivered = Receipt::count_undelivered(&mut tx, id).await?;
        if undelivered > 0 {
            return Ok(Delete::Undelivered(undelivered));
        }
        sqlx::query("DELETE FROM servers WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(Delete::Deleted)
    }
}

/// What [`Server::delete`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Delete {
    Deleted,
    NotFound,
    /// This many paid purchases from the server's store haven't been delivered; nothing was
    /// deleted.
    Undelivered(i64),
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
