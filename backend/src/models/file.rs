use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};

/// Owner of community files, which belong to no user (e.g. the community logo).
pub const COMMUNITY_OWNER: i64 = -1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "file_status", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FileStatus {
    /// Issued an upload target; the bytes haven't been verified yet.
    Pending,
    Ready,
}

/// An uploaded file (docs/STORAGE.md).
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct File {
    pub owner_id: i64,
    pub id: String,
    pub backend: String,
    pub key: String,
    pub name: String,
    pub content_type: String,
    pub size: i64,
    /// The provider's ETag when it was verified. Recorded for auditing; not used yet.
    #[allow(dead_code)]
    pub e_tag: Option<String>,
    pub status: FileStatus,
    pub uploaded_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    /// The support message it's attached to. Such files are only served to the ticket's
    /// creator (not for internal notes) and the Support Team.
    pub support_message_id: Option<i64>,
}

/// A file attached to a forum post or support message, as clients see it. Its URL is
/// `/file/{owner_id}/{id}`.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Attachment {
    pub owner_id: i64,
    pub id: String,
    pub name: String,
    pub content_type: String,
    pub size: i64,
}

/// What a file can be attached to.
#[derive(Clone, Copy, Debug)]
pub enum AttachTo {
    ForumPost(i64),
    SupportMessage(i64),
}

/// A JSON array of the files whose `$column` is `$id` (an SQL expression), oldest first;
/// decoded as `Json<Vec<Attachment>>`. A macro so queries stay `&'static str`.
macro_rules! attachments_json {
    ($column:literal, $id:expr) => {
        concat!(
            "COALESCE((SELECT json_agg(json_build_object(
                 'owner_id', f.owner_id, 'id', f.id, 'name', f.name,
                 'content_type', f.content_type, 'size', f.size) ORDER BY f.created_at, f.id)
               FROM files f WHERE f.", $column, " = ", $id, "), '[]'::json)"
        )
    };
}
pub(crate) use attachments_json;

/// A file about to be uploaded. Already validated (see `routes::files`).
pub struct NewFile<'a> {
    pub owner_id: i64,
    pub id: &'a str,
    pub backend: &'a str,
    pub key: &'a str,
    pub name: &'a str,
    pub content_type: &'a str,
    pub size: i64,
    pub uploaded_by: i64,
}

/// `$sql` followed by the file columns, then `$rest`.
macro_rules! with_columns {
    ($sql:literal $(, $rest:literal)?) => {
        concat!(
            $sql,
            " owner_id, id, backend, key, name, content_type, size, e_tag, status,
              uploaded_by, created_at, completed_at, support_message_id"
            $(, $rest)?
        )
    };
}

impl File {
    /// Object key of file `id` in `owner_id`'s folder.
    pub fn key_for(owner_id: i64, id: &str) -> String {
        format!("files/{owner_id}/{id}")
    }

    pub async fn create(db: &PgPool, new: &NewFile<'_>) -> Result<Self, sqlx::Error> {
        sqlx::query_as(with_columns!(
            "INSERT INTO files (owner_id, id, backend, key, name, content_type, size, uploaded_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING"
        ))
        .bind(new.owner_id)
        .bind(new.id)
        .bind(new.backend)
        .bind(new.key)
        .bind(new.name)
        .bind(new.content_type)
        .bind(new.size)
        .bind(new.uploaded_by)
        .fetch_one(db)
        .await
    }

    pub async fn get(db: &PgPool, owner_id: i64, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(with_columns!("SELECT", " FROM files WHERE owner_id = $1 AND id = $2"))
            .bind(owner_id)
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// How many uploads `user_id` started in the last `seconds`, and when the oldest of
    /// them started; for the rate limit.
    pub async fn recent_uploads_by(
        db: &PgPool,
        user_id: i64,
        seconds: f64,
    ) -> Result<(i64, Option<DateTime<Utc>>), sqlx::Error> {
        sqlx::query_as(
            "SELECT count(*), min(created_at) FROM files
             WHERE uploaded_by = $1 AND created_at > now() - make_interval(secs => $2)",
        )
        .bind(user_id)
        .bind(seconds)
        .fetch_one(db)
        .await
    }

    /// Whether every one of `ids` is a ready file in `owner_id`'s folder that isn't
    /// attached to anything yet.
    pub async fn attachable(db: &PgPool, owner_id: i64, ids: &[String]) -> Result<bool, sqlx::Error> {
        if ids.is_empty() {
            return Ok(true);
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM files
             WHERE owner_id = $1 AND id = ANY($2) AND status = 'READY'
               AND forum_post_id IS NULL AND support_message_id IS NULL",
        )
        .bind(owner_id)
        .bind(ids)
        .fetch_one(db)
        .await?;
        Ok(count == ids.len() as i64)
    }

    /// Attaches `ids` (files in `owner_id`'s folder) to `target`, returning them in the
    /// order given. Callers check [`File::attachable`] first; if a file has been taken or
    /// deleted since, this fails with `RowNotFound` so the caller's transaction rolls back.
    pub async fn attach(
        conn: &mut PgConnection,
        target: AttachTo,
        owner_id: i64,
        ids: &[String],
    ) -> Result<Vec<Attachment>, sqlx::Error> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let (sql, target_id) = match target {
            AttachTo::ForumPost(id) => (
                "UPDATE files SET forum_post_id = $1
                 WHERE owner_id = $2 AND id = ANY($3) AND status = 'READY'
                   AND forum_post_id IS NULL AND support_message_id IS NULL
                 RETURNING owner_id, id, name, content_type, size",
                id,
            ),
            AttachTo::SupportMessage(id) => (
                "UPDATE files SET support_message_id = $1
                 WHERE owner_id = $2 AND id = ANY($3) AND status = 'READY'
                   AND forum_post_id IS NULL AND support_message_id IS NULL
                 RETURNING owner_id, id, name, content_type, size",
                id,
            ),
        };
        let mut attached: Vec<Attachment> = sqlx::query_as(sql)
            .bind(target_id)
            .bind(owner_id)
            .bind(ids)
            .fetch_all(conn)
            .await?;
        if attached.len() != ids.len() {
            return Err(sqlx::Error::RowNotFound);
        }
        attached.sort_by_key(|a| ids.iter().position(|id| *id == a.id));
        Ok(attached)
    }

    /// The files attached to a forum post.
    pub async fn of_forum_post(db: &PgPool, post_id: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(with_columns!("SELECT", " FROM files WHERE forum_post_id = $1"))
            .bind(post_id)
            .fetch_all(db)
            .await
    }

    /// The files attached to any post of a forum thread.
    pub async fn of_forum_thread(db: &PgPool, thread_id: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(with_columns!(
            "SELECT",
            " FROM files
              WHERE forum_post_id IN (SELECT id FROM forum_posts WHERE thread_id = $1)"
        ))
        .bind(thread_id)
        .fetch_all(db)
        .await
    }

    /// Marks a pending file ready. `None` if it's no longer pending (already completed, or
    /// deleted meanwhile).
    pub async fn mark_ready(
        db: &PgPool,
        owner_id: i64,
        id: &str,
        e_tag: Option<&str>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as(with_columns!(
            "UPDATE files SET status = 'READY', e_tag = $3, completed_at = now()
             WHERE owner_id = $1 AND id = $2 AND status = 'PENDING'
             RETURNING"
        ))
        .bind(owner_id)
        .bind(id)
        .bind(e_tag)
        .fetch_optional(db)
        .await
    }

    pub async fn delete(db: &PgPool, owner_id: i64, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM files WHERE owner_id = $1 AND id = $2")
            .bind(owner_id)
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Deletes a file only if it's still pending, so a sweep can't race its completion.
    pub async fn delete_pending(db: &PgPool, owner_id: i64, id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM files WHERE owner_id = $1 AND id = $2 AND status = 'PENDING'")
            .bind(owner_id)
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Pending files started more than `minutes` ago: abandoned uploads.
    pub async fn stale_pending(db: &PgPool, minutes: i64) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(with_columns!(
            "SELECT",
            " FROM files
             WHERE status = 'PENDING' AND created_at < now() - make_interval(mins => $1)
             ORDER BY created_at LIMIT 500"
        ))
        .bind(minutes as i32)
        .fetch_all(db)
        .await
    }
}
