//! Private messages (docs/PRIVATE_MESSAGES.md). There is no chat entity: a chat is the
//! pair of users, addressed by the other user's id.
//!
//! What a user sees is filtered here, never deleted: a message is hidden from a user once
//! they've deleted the chat (their side's flag), and from both once it is temporary and
//! more than 24 hours old. The admin variants skip both filters and flag such
//! messages instead.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, QueryBuilder, types::Json};

use super::{
    cursor::{Cursor, Page},
    file::{AttachTo, Attachment, File, attachments_json},
    support::{UserRef, user_json},
};

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct PrivateMessage {
    pub id: i64,
    pub author_id: i64,
    pub recipient_id: i64,
    /// Markdown.
    pub content: String,
    /// Hidden from both users 24 hours after `created_at`.
    pub temporary: bool,
    /// `None` = the recipient hasn't opened the chat since.
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// Files attached when the message was sent.
    pub attachments: Json<Vec<Attachment>>,
}

/// A message as staff see it: including deleted and expired ones, flagged.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct AdminMessage {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub message: PrivateMessage,
    pub deleted_by_author: bool,
    pub deleted_by_recipient: bool,
    /// Either user deleted the chat since it was sent.
    pub deleted: bool,
    /// Temporary and over 24 hours old, so hidden from both users.
    pub expired: bool,
}

/// One entry of a user's chat list: the other user and the chat's last message.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Chat {
    /// The other user.
    pub user: Json<UserRef>,
    pub last_message_id: i64,
    pub last_author_id: i64,
    /// The last message's content. Markdown as stored; the route turns it into a short
    /// plain-text preview.
    pub preview: String,
    pub last_at: DateTime<Utc>,
    /// Messages the other user sent that this user hasn't read.
    pub unread_count: i64,
}

impl Chat {
    pub fn cursor(&self) -> Cursor {
        Cursor {
            created_at: self.last_at,
            id: self.last_message_id,
        }
    }
}

/// A message to send. Already validated (see `routes::messages`).
pub struct NewMessage {
    pub content: String,
    /// File ids of the author's to attach.
    pub attachments: Vec<String>,
    pub temporary: bool,
}

/// SQL: whether message `$m` is temporary and over 24 hours old, so hidden from both users.
macro_rules! expired {
    ($m:literal) => {
        concat!(
            "(",
            $m,
            ".temporary AND ",
            $m,
            ".created_at < now() - interval '24 hours')"
        )
    };
}

macro_rules! message_columns {
    ($m:literal) => {
        concat!(
            $m,
            ".id, ",
            $m,
            ".author_id, ",
            $m,
            ".recipient_id, ",
            $m,
            ".content, ",
            $m,
            ".temporary, ",
            $m,
            ".read_at, ",
            $m,
            ".created_at, ",
            attachments_json!("private_message_id", concat!($m, ".id")),
            " AS attachments"
        )
    };
}

macro_rules! admin_columns {
    ($m:literal) => {
        concat!(
            message_columns!($m),
            ", ",
            $m,
            ".deleted_by_author, ",
            $m,
            ".deleted_by_recipient, (",
            $m,
            ".deleted_by_author OR ",
            $m,
            ".deleted_by_recipient) AS deleted, ",
            expired!($m),
            " AS expired"
        )
    };
}

/// Whose view a query takes.
#[derive(Clone, Copy, Debug)]
enum View {
    /// A user's own: without the messages they deleted or that expired.
    User,
    /// Staff's: everything.
    Admin,
}

impl PrivateMessage {
    pub fn cursor(&self) -> Cursor {
        Cursor {
            created_at: self.created_at,
            id: self.id,
        }
    }

    /// Sends a message and attaches its files, in one transaction. Callers check
    /// `File::attachable` first; a file taken since fails the whole send.
    pub async fn insert(
        db: &PgPool,
        author_id: i64,
        recipient_id: i64,
        new: &NewMessage,
    ) -> sqlx::Result<Self> {
        let mut tx = db.begin().await?;
        let mut message: Self = sqlx::query_as(concat!(
            "INSERT INTO private_messages (author_id, recipient_id, content, temporary)
             VALUES ($1, $2, $3, $4)
             RETURNING ",
            message_columns!("private_messages")
        ))
        .bind(author_id)
        .bind(recipient_id)
        .bind(&new.content)
        .bind(new.temporary)
        .fetch_one(&mut *tx)
        .await?;
        message.attachments = Json(
            File::attach(
                &mut tx,
                AttachTo::PrivateMessage(message.id),
                author_id,
                &new.attachments,
            )
            .await?,
        );
        tx.commit().await?;
        Ok(message)
    }

    /// How many messages `author_id` sent in the last `seconds`, and when the oldest of
    /// them was sent; for the rate limit.
    pub async fn recent_sent_by(
        db: &PgPool,
        author_id: i64,
        seconds: f64,
    ) -> sqlx::Result<(i64, Option<DateTime<Utc>>)> {
        sqlx::query_as(
            "SELECT count(*), min(created_at) FROM private_messages
             WHERE author_id = $1 AND created_at > now() - make_interval(secs => $2)",
        )
        .bind(author_id)
        .bind(seconds)
        .fetch_one(db)
        .await
    }

    /// One page of the chat between `viewer` and `other` as `viewer` sees it, newest first.
    pub async fn thread(
        db: &PgPool,
        viewer: i64,
        other: i64,
        after: Option<Cursor>,
        limit: i64,
    ) -> sqlx::Result<Page<Self>> {
        let rows: Vec<Self> = thread_query(
            concat!("SELECT ", message_columns!("m")),
            View::User,
            viewer,
            other,
            after,
            limit,
        )
        .build_query_as()
        .fetch_all(db)
        .await?;
        Ok(Page::new(rows, limit, Self::cursor))
    }

    /// Like [`PrivateMessage::thread`] for staff: every message between `a` and `b`,
    /// deleted and expired ones included.
    pub async fn admin_thread(
        db: &PgPool,
        a: i64,
        b: i64,
        after: Option<Cursor>,
        limit: i64,
    ) -> sqlx::Result<Page<AdminMessage>> {
        let rows: Vec<AdminMessage> = thread_query(
            concat!("SELECT ", admin_columns!("m")),
            View::Admin,
            a,
            b,
            after,
            limit,
        )
        .build_query_as()
        .fetch_all(db)
        .await?;
        Ok(Page::new(rows, limit, |m| m.message.cursor()))
    }

    /// Sets `read_at` on everything `author_id` sent `recipient_id` that's still unread.
    /// Returns how many there were.
    pub async fn mark_read(db: &PgPool, recipient_id: i64, author_id: i64) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "UPDATE private_messages SET read_at = now()
              WHERE recipient_id = $1 AND author_id = $2 AND read_at IS NULL",
        )
        .bind(recipient_id)
        .bind(author_id)
        .execute(db)
        .await?;
        Ok(result.rows_affected())
    }

    /// Deletes the chat with `other` for `user_id` only: sets their side's flag on the
    /// messages that exist now. Later messages aren't flagged, so they start a fresh chat.
    /// Returns how many messages were hidden.
    pub async fn delete_chat(db: &PgPool, user_id: i64, other: i64) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "UPDATE private_messages
                SET deleted_by_author = deleted_by_author OR author_id = $1,
                    deleted_by_recipient = deleted_by_recipient OR recipient_id = $1
              WHERE (author_id = $1 AND recipient_id = $2 AND NOT deleted_by_author)
                 OR (author_id = $2 AND recipient_id = $1 AND NOT deleted_by_recipient)",
        )
        .bind(user_id)
        .bind(other)
        .execute(db)
        .await?;
        Ok(result.rows_affected())
    }

    /// How many messages the user can see that they haven't read, for the navigation badge.
    pub async fn unread_count(db: &PgPool, user_id: i64) -> sqlx::Result<i64> {
        sqlx::query_scalar(concat!(
            "SELECT count(*) FROM private_messages m
              WHERE m.recipient_id = $1 AND m.read_at IS NULL AND NOT m.deleted_by_recipient
                AND NOT ",
            expired!("m")
        ))
        .bind(user_id)
        .fetch_one(db)
        .await
    }

    /// Whether the user has ever sent a message, including ones since deleted or expired.
    /// Until they have, the message page explains that staff can read messages.
    pub async fn has_sent(db: &PgPool, user_id: i64) -> sqlx::Result<bool> {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM private_messages WHERE author_id = $1)")
            .bind(user_id)
            .fetch_one(db)
            .await
    }
}

impl Chat {
    /// One page of the user's chat list, the most recent chat first. Only chats with a
    /// message the user can still see.
    pub async fn list_for(
        db: &PgPool,
        user_id: i64,
        after: Option<Cursor>,
        limit: i64,
    ) -> sqlx::Result<Page<Self>> {
        chats(db, View::User, user_id, after, limit).await
    }

    /// Like [`Chat::list_for`] for staff: every chat the user has, deleted and expired
    /// messages included.
    pub async fn admin_list_for(
        db: &PgPool,
        user_id: i64,
        after: Option<Cursor>,
        limit: i64,
    ) -> sqlx::Result<Page<Self>> {
        chats(db, View::Admin, user_id, after, limit).await
    }
}

/// The messages between `a` and `b` (`a` being the viewer in [`View::User`]), newest first,
/// after the cursor, with one extra row to tell whether there is a next page.
fn thread_query(
    select: &'static str,
    view: View,
    a: i64,
    b: i64,
    after: Option<Cursor>,
    limit: i64,
) -> QueryBuilder<Postgres> {
    let mut q: QueryBuilder<Postgres> = QueryBuilder::new(select);
    q.push(" FROM private_messages m WHERE ((m.author_id = ")
        .push_bind(a)
        .push(" AND m.recipient_id = ")
        .push_bind(b);
    if let View::User = view {
        q.push(" AND NOT m.deleted_by_author");
    }
    q.push(") OR (m.author_id = ")
        .push_bind(b)
        .push(" AND m.recipient_id = ")
        .push_bind(a);
    if let View::User = view {
        q.push(" AND NOT m.deleted_by_recipient");
    }
    q.push("))");
    if let View::User = view {
        q.push(concat!(" AND NOT ", expired!("m")));
    }
    if let Some(cursor) = after {
        q.push(" AND (m.created_at, m.id) < (")
            .push_bind(cursor.created_at)
            .push(", ")
            .push_bind(cursor.id)
            .push(")");
    }
    q.push(" ORDER BY m.created_at DESC, m.id DESC LIMIT ")
        .push_bind(limit + 1);
    q
}

/// The user's chats: the last message with each other user, newest first.
async fn chats(
    db: &PgPool,
    view: View,
    user_id: i64,
    after: Option<Cursor>,
    limit: i64,
) -> sqlx::Result<Page<Chat>> {
    let mut q: QueryBuilder<Postgres> = QueryBuilder::new(
        "WITH mine AS (
             SELECT m.id, m.author_id, m.recipient_id, m.content, m.read_at, m.created_at,
                    CASE WHEN m.author_id = ",
    );
    q.push_bind(user_id)
        .push(
            " THEN m.recipient_id ELSE m.author_id END AS other_id
             FROM private_messages m
             WHERE ((m.author_id = ",
        )
        .push_bind(user_id);
    if let View::User = view {
        q.push(" AND NOT m.deleted_by_author");
    }
    q.push(") OR (m.recipient_id = ").push_bind(user_id);
    if let View::User = view {
        q.push(" AND NOT m.deleted_by_recipient");
    }
    q.push("))");
    if let View::User = view {
        q.push(concat!(" AND NOT ", expired!("m")));
    }
    // Window functions run before DISTINCT ON, so the count covers the whole chat.
    q.push(
        "), last AS (
             SELECT DISTINCT ON (other_id) *,
                    count(*) FILTER (WHERE recipient_id = ",
    )
    .push_bind(user_id)
    .push(concat!(
        " AND read_at IS NULL) OVER (PARTITION BY other_id) AS unread_count
             FROM mine
             ORDER BY other_id, created_at DESC, id DESC
         )
         SELECT ",
        user_json!("u"),
        " AS user, l.id AS last_message_id, l.author_id AS last_author_id,
                l.content AS preview, l.created_at AS last_at, l.unread_count
         FROM last l JOIN users u ON u.id = l.other_id
         WHERE true"
    ));
    if let Some(cursor) = after {
        q.push(" AND (l.created_at, l.id) < (")
            .push_bind(cursor.created_at)
            .push(", ")
            .push_bind(cursor.id)
            .push(")");
    }
    q.push(" ORDER BY l.created_at DESC, l.id DESC LIMIT ")
        .push_bind(limit + 1);
    let rows: Vec<Chat> = q.build_query_as().fetch_all(db).await?;
    Ok(Page::new(rows, limit, Chat::cursor))
}

/// See [`PrivateMessage::participants`].
#[derive(Clone, Copy, Debug, sqlx::FromRow)]
pub struct Participants {
    pub author_id: i64,
    pub recipient_id: i64,
}

impl Participants {
    /// Whether the user is one of the two, so may see the message (and its files) whatever
    /// they've deleted or whether it expired.
    pub fn includes(&self, user_id: i64) -> bool {
        self.author_id == user_id || self.recipient_id == user_id
    }
}

impl PrivateMessage {
    /// The two users of message `id`, for its attachments' read rule. `None` if there is no
    /// such message.
    pub async fn participants(db: &PgPool, id: i64) -> sqlx::Result<Option<Participants>> {
        sqlx::query_as("SELECT author_id, recipient_id FROM private_messages WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn participants() {
        let p = Participants {
            author_id: 3,
            recipient_id: 7,
        };
        assert!(p.includes(3));
        assert!(p.includes(7));
        assert!(!p.includes(4), "only the two users in the chat");
    }

    #[test]
    fn expiry() {
        assert_eq!(
            expired!("m"),
            "(m.temporary AND m.created_at < now() - interval '24 hours')"
        );
    }
}
