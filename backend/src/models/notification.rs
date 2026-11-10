//! Notifications (docs/NOTIFICATIONS.md): one row per recipient, telling them something
//! happened and linking to where they can act on it. `crate::notify` creates them.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "notification_type", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationType {
    Support,
    Store,
    Forum,
    /// Punishments, usergroup changes.
    Account,
    /// Anything else, e.g. admin messages.
    System,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Notification {
    pub id: i64,
    /// The recipient.
    pub user_id: i64,
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub kind: NotificationType,
    /// Plain text.
    pub title: String,
    /// Plain text.
    pub body: Option<String>,
    /// Site-relative path the notification links to.
    pub url: Option<String>,
    pub created_at: DateTime<Utc>,
    /// `None` = unread.
    pub read_at: Option<DateTime<Utc>>,
}

pub const MAX_TITLE: usize = 128;
pub const MAX_BODY: usize = 512;

/// Read notifications are deleted once they're this old.
pub const READ_RETENTION_DAYS: i32 = 30;
/// Every notification is deleted once it's this old.
pub const RETENTION_DAYS: i32 = 90;
/// How many notifications a user keeps; beyond it the oldest read ones go first.
pub const MAX_PER_USER: i64 = 200;

/// A notification to write. `title` and `body` must already fit [`MAX_TITLE`] and
/// [`MAX_BODY`], and `url` must be a site-relative path (the table checks all three).
pub struct NewNotification {
    pub kind: NotificationType,
    /// The kind of event, e.g. `thread_reply`. Repeats of the same event about the same
    /// `url` collapse into one unread notification.
    pub event: &'static str,
    pub title: String,
    pub body: Option<String>,
    pub url: Option<String>,
}

/// Where a page of notifications ended: the `created_at` and `id` of its last row. Sent to
/// clients as an opaque string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    created_at: DateTime<Utc>,
    id: i64,
}

impl Cursor {
    pub fn parse(value: &str) -> Option<Self> {
        let (micros, id) = value.split_once('-')?;
        Some(Self {
            created_at: DateTime::from_timestamp_micros(micros.parse().ok()?)?,
            id: id.parse().ok()?,
        })
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Postgres timestamps have microsecond precision, so this round-trips exactly.
        write!(f, "{}-{}", self.created_at.timestamp_micros(), self.id)
    }
}

/// Which of a user's notifications to list.
#[derive(Debug)]
pub struct NotificationFilter {
    pub unread_only: bool,
    pub kind: Option<NotificationType>,
    pub limit: i64,
    /// Only notifications after this one in the list (older than it).
    pub after: Option<Cursor>,
}

#[derive(Debug, Serialize)]
pub struct NotificationPage {
    pub notifications: Vec<Notification>,
    /// Pass as `cursor` for the next page; `None` on the last page.
    pub next_cursor: Option<String>,
}

macro_rules! columns {
    () => {
        "id, user_id, type, title, body, url, created_at, read_at"
    };
}

impl Notification {
    /// Writes the notification for each recipient (ids with no user are skipped). A
    /// recipient who already has an unread notification with the same type, event and url
    /// gets that one updated and moved to the top instead. Returns how many rows were
    /// written.
    pub async fn create(db: &PgPool, recipients: &[i64], new: &NewNotification) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "INSERT INTO notifications (user_id, type, event, title, body, url)
             SELECT u.id, $2, $3, $4, $5, $6 FROM users u WHERE u.id = ANY($1)
             ON CONFLICT (user_id, type, event, url) WHERE read_at IS NULL AND url IS NOT NULL
             DO UPDATE SET title = EXCLUDED.title, body = EXCLUDED.body,
                           created_at = EXCLUDED.created_at",
        )
        .bind(recipients)
        .bind(new.kind)
        .bind(new.event)
        .bind(&new.title)
        .bind(&new.body)
        .bind(&new.url)
        .execute(db)
        .await?;
        Ok(result.rows_affected())
    }

    /// One page of the user's notifications, newest first.
    pub async fn list(db: &PgPool, user_id: i64, filter: &NotificationFilter) -> sqlx::Result<NotificationPage> {
        let mut q: QueryBuilder<Postgres> =
            QueryBuilder::new(concat!("SELECT ", columns!(), " FROM notifications WHERE user_id = "));
        q.push_bind(user_id);
        if filter.unread_only {
            q.push(" AND read_at IS NULL");
        }
        if let Some(kind) = filter.kind {
            q.push(" AND type = ").push_bind(kind);
        }
        if let Some(cursor) = filter.after {
            q.push(" AND (created_at, id) < (")
                .push_bind(cursor.created_at)
                .push(", ")
                .push_bind(cursor.id)
                .push(")");
        }
        // One extra row tells us whether there is a next page.
        q.push(" ORDER BY created_at DESC, id DESC LIMIT ").push_bind(filter.limit + 1);
        let mut notifications: Vec<Notification> = q.build_query_as().fetch_all(db).await?;
        let has_more = notifications.len() as i64 > filter.limit;
        notifications.truncate(filter.limit as usize);
        let next_cursor = notifications
            .last()
            .filter(|_| has_more)
            .map(|last| Cursor { created_at: last.created_at, id: last.id }.to_string());
        Ok(NotificationPage { notifications, next_cursor })
    }

    pub async fn unread_count(db: &PgPool, user_id: i64) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT count(*) FROM notifications WHERE user_id = $1 AND read_at IS NULL")
            .bind(user_id)
            .fetch_one(db)
            .await
    }

    /// Marks the user's notification read, keeping the time it was first read. `None` if
    /// the user has no such notification.
    pub async fn mark_read(db: &PgPool, user_id: i64, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!(
            "UPDATE notifications SET read_at = COALESCE(read_at, now())
              WHERE id = $1 AND user_id = $2
              RETURNING ", columns!()
        ))
        .bind(id)
        .bind(user_id)
        .fetch_optional(db)
        .await
    }

    /// Marks every unread notification of the user's read, or only those of `kind`.
    /// Returns how many were unread.
    pub async fn mark_all_read(db: &PgPool, user_id: i64, kind: Option<NotificationType>) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "UPDATE notifications SET read_at = now()
              WHERE user_id = $1 AND read_at IS NULL AND ($2::notification_type IS NULL OR type = $2)",
        )
        .bind(user_id)
        .bind(kind)
        .execute(db)
        .await?;
        Ok(result.rows_affected())
    }

    /// False if the user has no such notification.
    pub async fn delete(db: &PgPool, user_id: i64, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM notifications WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Deletes every read notification of the user's. Returns how many there were.
    pub async fn delete_read(db: &PgPool, user_id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query("DELETE FROM notifications WHERE user_id = $1 AND read_at IS NOT NULL")
            .bind(user_id)
            .execute(db)
            .await?;
        Ok(result.rows_affected())
    }

    /// Applies retention: deletes read notifications older than [`READ_RETENTION_DAYS`],
    /// any older than [`RETENTION_DAYS`], and whatever a user has beyond [`MAX_PER_USER`]
    /// (their oldest read ones first, then their oldest unread). Returns how many were
    /// deleted.
    pub async fn sweep(db: &PgPool) -> sqlx::Result<u64> {
        let expired = sqlx::query(
            "DELETE FROM notifications
              WHERE created_at < now() - make_interval(days => $1)
                 OR (read_at IS NOT NULL AND created_at < now() - make_interval(days => $2))",
        )
        .bind(RETENTION_DAYS)
        .bind(READ_RETENTION_DAYS)
        .execute(db)
        .await?;
        let over_limit = sqlx::query(
            "DELETE FROM notifications
              WHERE id IN (
                    SELECT id FROM (
                        SELECT id, row_number() OVER (
                                   PARTITION BY user_id
                                   ORDER BY (read_at IS NULL) DESC, created_at DESC, id DESC) AS rank
                        FROM notifications
                        WHERE user_id IN (SELECT user_id FROM notifications
                                          GROUP BY user_id HAVING count(*) > $1)
                    ) ranked
                    WHERE rank > $1)",
        )
        .bind(MAX_PER_USER)
        .execute(db)
        .await?;
        Ok(expired.rows_affected() + over_limit.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursors() {
        let cursor = Cursor {
            created_at: DateTime::from_timestamp_micros(1_794_000_000_123_456).unwrap(),
            id: 42,
        };
        assert_eq!(cursor.to_string(), "1794000000123456-42");
        assert_eq!(Cursor::parse(&cursor.to_string()), Some(cursor));
        assert_eq!(Cursor::parse(""), None);
        assert_eq!(Cursor::parse("42"), None);
        assert_eq!(Cursor::parse("abc-1"), None);
        assert_eq!(Cursor::parse("1794000000123456-"), None);
    }
}
