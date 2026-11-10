//! The support system (docs/SUPPORT.MD): tickets hold messages between a user and the
//! support team; FAQs answer common questions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool, Postgres, QueryBuilder, types::Json};

use super::file::{AttachTo, Attachment, File, attachments_json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "ticket_category", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TicketCategory {
    Other,
    /// Involves other users/players.
    UserReport,
    Suggestion,
    BugReport,
    /// Didn't receive a store item, etc.
    UserIssue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "support_status", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupportStatus {
    /// Just created; only the initial message.
    Open,
    /// The support team replied and is waiting on the user.
    AwaitingUser,
    /// The user replied and is waiting on the support team.
    AwaitingAdmin,
    Resolved,
    /// No continuation, with no resolution (includes automatic closing).
    Closed,
}

impl SupportStatus {
    /// Resolved and closed tickets are done: only the support team can continue them.
    pub fn is_done(self) -> bool {
        matches!(self, Self::Resolved | Self::Closed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "faq_category", rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaqCategory {
    General,
    Support,
    Store,
    Forum,
    User,
    Community,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct SupportTicket {
    pub id: i64,
    pub title: String,
    /// Short description, max 256 characters.
    pub subject: String,
    pub user_id: i64,
    /// Other users the ticket involves.
    pub associated_user_ids: Vec<i64>,
    pub initial_message_id: i64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub category: TicketCategory,
    pub status: SupportStatus,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    /// Who resolved or closed it; `None` when closed automatically.
    pub closed_by: Option<i64>,
    /// Support team members who have responded to or changed the ticket.
    pub responders: Vec<i64>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct SupportMessage {
    pub id: i64,
    pub author_id: i64,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    /// Markdown.
    pub content: String,
    /// What the message said before its one allowed edit.
    pub previous_content: Option<String>,
    pub ticket_id: i64,
    /// Only the support team can see internal messages.
    pub is_internal: bool,
    /// Files attached when the message was written.
    pub attachments: Json<Vec<Attachment>>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Faq {
    pub id: i64,
    pub title: String,
    pub subject: String,
    pub category: FaqCategory,
    /// Markdown.
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub created_by: i64,
    /// Everyone who has edited it, in order of their first edit.
    pub editors: Vec<i64>,
}

/// The parts of a user the support pages display next to tickets, messages and FAQs.
#[derive(Clone, Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRef {
    pub id: i64,
    pub username: String,
    pub avatar_url: String,
}

/// A ticket as listed on the support pages.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct TicketSummary {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub ticket: SupportTicket,
    pub author: Json<UserRef>,
    /// Messages the user can see (internal ones excluded).
    pub message_count: i64,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct MessageWithAuthor {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub message: SupportMessage,
    pub author: Json<UserRef>,
}

/// An FAQ with the people who wrote and edited it.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct FaqWithPeople {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub faq: Faq,
    pub creator: Json<UserRef>,
    pub editor_users: Json<Vec<UserRef>>,
}

/// Already validated (see `routes::support`).
pub struct TicketDetails {
    pub title: String,
    pub subject: String,
    pub category: TicketCategory,
    pub associated_user_ids: Vec<i64>,
    pub content: String,
    /// Files of the user's to attach to the initial message.
    pub attachments: Vec<String>,
}

/// Already validated (see `routes::support`).
pub struct FaqDetails {
    pub title: String,
    pub subject: String,
    pub category: FaqCategory,
    pub description: String,
}

/// Order of ticket lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TicketSort {
    /// Most recently updated first.
    #[default]
    Latest,
    /// Least recently updated first.
    Oldest,
    /// Newest ticket first.
    Created,
}

/// Filters for ticket lists. Every filter is optional; together they must all match.
#[derive(Debug, Default)]
pub struct TicketFilter {
    /// Only this user's tickets.
    pub user_id: Option<i64>,
    pub status: Option<SupportStatus>,
    /// `true` for tickets still in progress, `false` for resolved/closed ones.
    pub active: Option<bool>,
    pub category: Option<TicketCategory>,
    /// Only tickets updated within this many days.
    pub updated_within_days: Option<i32>,
    pub sort: TicketSort,
}

/// Tickets in `AWAITING_USER` with no activity for this long are closed automatically.
pub const AUTO_CLOSE_AFTER_DAYS: i32 = 7;

/// How many tickets a user may have in progress at once.
pub const MAX_ACTIVE_TICKETS: i64 = 5;

/// A JSON object of the user aliased `u`, decoded as a [`UserRef`].
macro_rules! user_json {
    ($alias:literal) => {
        concat!(
            "json_build_object('id', ", $alias, ".id, 'username', ", $alias, ".username,
                               'avatar_url', ", $alias, ".avatar_url)"
        )
    };
}

macro_rules! ticket_columns {
    ($t:literal) => {
        concat!(
            $t, ".id, ", $t, ".title, ", $t, ".subject, ", $t, ".user_id, ",
            $t, ".associated_user_ids, ", $t, ".initial_message_id, ", $t, ".created_at, ",
            $t, ".last_updated, ", $t, ".category, ", $t, ".status, ", $t, ".resolved_at, ",
            $t, ".closed_at, ", $t, ".closed_by, ", $t, ".responders"
        )
    };
}

macro_rules! message_columns {
    ($m:literal) => {
        concat!(
            $m, ".id, ", $m, ".author_id, ", $m, ".created_at, ", $m, ".last_updated, ",
            $m, ".content, ", $m, ".previous_content, ", $m, ".ticket_id, ", $m, ".is_internal, ",
            attachments_json!("support_message_id", concat!($m, ".id")), " AS attachments"
        )
    };
}

macro_rules! faq_columns {
    ($f:literal) => {
        concat!(
            $f, ".id, ", $f, ".title, ", $f, ".subject, ", $f, ".category, ", $f, ".description, ",
            $f, ".created_at, ", $f, ".last_updated, ", $f, ".created_by, ", $f, ".editors"
        )
    };
}

/// Selects FAQ columns plus their creator and editors as JSON, from `faqs f`.
macro_rules! select_faq {
    ($rest:literal) => {
        concat!(
            "SELECT ", faq_columns!("f"), ", ", user_json!("u"), " AS creator,
                    (SELECT COALESCE(json_agg(", user_json!("e"), " ORDER BY ids.ord), '[]'::json)
                     FROM unnest(f.editors) WITH ORDINALITY AS ids(id, ord)
                     JOIN users e ON e.id = ids.id) AS editor_users
             FROM faqs f JOIN users u ON u.id = f.created_by ",
            $rest
        )
    };
}

impl SupportTicket {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!(
            "SELECT ", ticket_columns!("t"), " FROM support_tickets t WHERE t.id = $1"
        ))
        .bind(id)
        .fetch_optional(db)
        .await
    }

    /// Tickets matching every filter. `include_internal` counts internal messages too.
    pub async fn list(
        db: &PgPool,
        filter: &TicketFilter,
        include_internal: bool,
    ) -> sqlx::Result<Vec<TicketSummary>> {
        let mut q: QueryBuilder<Postgres> = QueryBuilder::new(concat!(
            "SELECT ", ticket_columns!("t"), ", ", user_json!("u"), " AS author,
                    (SELECT count(*) FROM support_messages m
                     WHERE m.ticket_id = t.id AND ("
        ));
        q.push_bind(include_internal)
            .push(
                " OR NOT m.is_internal)) AS message_count
             FROM support_tickets t JOIN users u ON u.id = t.user_id
             WHERE true",
            );
        if let Some(user_id) = filter.user_id {
            q.push(" AND t.user_id = ").push_bind(user_id);
        }
        if let Some(status) = filter.status {
            q.push(" AND t.status = ").push_bind(status);
        }
        if let Some(active) = filter.active {
            q.push(if active {
                " AND t.status NOT IN ('RESOLVED', 'CLOSED')"
            } else {
                " AND t.status IN ('RESOLVED', 'CLOSED')"
            });
        }
        if let Some(category) = filter.category {
            q.push(" AND t.category = ").push_bind(category);
        }
        if let Some(days) = filter.updated_within_days {
            q.push(" AND t.last_updated > now() - make_interval(days => ")
                .push_bind(days)
                .push(")");
        }
        q.push(match filter.sort {
            TicketSort::Latest => " ORDER BY t.last_updated DESC, t.id DESC",
            TicketSort::Oldest => " ORDER BY t.last_updated ASC, t.id ASC",
            TicketSort::Created => " ORDER BY t.created_at DESC, t.id DESC",
        });
        q.build_query_as().fetch_all(db).await
    }

    /// How many of the user's tickets are still in progress.
    pub async fn count_active(db: &PgPool, user_id: i64) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "SELECT count(*) FROM support_tickets
             WHERE user_id = $1 AND status NOT IN ('RESOLVED', 'CLOSED')",
        )
        .bind(user_id)
        .fetch_one(db)
        .await
    }

    /// Creates a ticket and its initial message together.
    pub async fn create(db: &PgPool, user_id: i64, details: &TicketDetails) -> sqlx::Result<Self> {
        let mut tx = db.begin().await?;
        // The ticket references its first message (a deferred foreign key), so reserve the
        // message's id before inserting either.
        let message_id: i64 =
            sqlx::query_scalar("SELECT nextval(pg_get_serial_sequence('support_messages', 'id'))")
                .fetch_one(&mut *tx)
                .await?;
        let ticket: Self = sqlx::query_as(concat!(
            "INSERT INTO support_tickets
                (title, subject, user_id, associated_user_ids, initial_message_id, category)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING ", ticket_columns!("support_tickets")
        ))
        .bind(&details.title)
        .bind(&details.subject)
        .bind(user_id)
        .bind(&details.associated_user_ids)
        .bind(message_id)
        .bind(details.category)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO support_messages (id, author_id, ticket_id, content, created_at, last_updated)
             VALUES ($1, $2, $3, $4, $5, $5)",
        )
        .bind(message_id)
        .bind(user_id)
        .bind(ticket.id)
        .bind(&details.content)
        .bind(ticket.created_at)
        .execute(&mut *tx)
        .await?;
        File::attach(&mut tx, AttachTo::SupportMessage(message_id), user_id, &details.attachments).await?;
        tx.commit().await?;
        Ok(ticket)
    }

    /// The ticket's messages, oldest first. Internal messages only when `include_internal`.
    pub async fn messages(&self, db: &PgPool, include_internal: bool) -> sqlx::Result<Vec<MessageWithAuthor>> {
        sqlx::query_as(concat!(
            "SELECT ", message_columns!("m"), ", ", user_json!("u"), " AS author
             FROM support_messages m JOIN users u ON u.id = m.author_id
             WHERE m.ticket_id = $1 AND ($2 OR NOT m.is_internal)
             ORDER BY m.created_at, m.id"
        ))
        .bind(self.id)
        .bind(include_internal)
        .fetch_all(db)
        .await
    }

    /// Adds a message from the ticket's owner and moves the ticket to `AWAITING_ADMIN`.
    pub async fn reply_as_user(
        &self,
        db: &PgPool,
        content: &str,
        attachments: &[String],
    ) -> sqlx::Result<SupportMessage> {
        let mut tx = db.begin().await?;
        let message = insert_message(&mut tx, self.id, self.user_id, content, false, attachments).await?;
        sqlx::query(
            "UPDATE support_tickets SET status = 'AWAITING_ADMIN', last_updated = $2 WHERE id = $1",
        )
        .bind(self.id)
        .bind(message.created_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(message)
    }

    /// Adds a message from a support team member, recording them as a responder. A visible
    /// reply moves the ticket to `AWAITING_USER` (re-opening it if it was done); an internal
    /// note changes nothing else.
    pub async fn reply_as_support(
        &self,
        db: &PgPool,
        author_id: i64,
        content: &str,
        is_internal: bool,
        attachments: &[String],
    ) -> sqlx::Result<SupportMessage> {
        let mut tx = db.begin().await?;
        let message = insert_message(&mut tx, self.id, author_id, content, is_internal, attachments).await?;
        if is_internal {
            add_responder(&mut tx, self.id, author_id).await?;
        } else {
            sqlx::query(
                "UPDATE support_tickets
                    SET status = 'AWAITING_USER', last_updated = $2,
                        resolved_at = NULL, closed_at = NULL, closed_by = NULL,
                        responders = CASE WHEN $3 = ANY(responders) THEN responders
                                          ELSE array_append(responders, $3) END
                  WHERE id = $1",
            )
            .bind(self.id)
            .bind(message.created_at)
            .bind(author_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(message)
    }

    /// Sets the status on behalf of support team member `by`, recording them as a
    /// responder. `RESOLVED` and `CLOSED` stamp their timestamps and `closed_by`; any other
    /// status re-opens the ticket and clears them.
    pub async fn set_status(&self, db: &PgPool, status: SupportStatus, by: i64) -> sqlx::Result<Self> {
        let mut tx = db.begin().await?;
        add_responder(&mut tx, self.id, by).await?;
        let ticket = sqlx::query_as(concat!(
            "UPDATE support_tickets
                SET status = $2,
                    last_updated = now(),
                    resolved_at = CASE WHEN $2 = 'RESOLVED' THEN now() ELSE NULL END,
                    closed_at = CASE WHEN $2 = 'CLOSED' THEN now() ELSE NULL END,
                    closed_by = CASE WHEN $2 IN ('RESOLVED', 'CLOSED') THEN $3 ELSE NULL END
              WHERE id = $1
              RETURNING ", ticket_columns!("support_tickets")
        ))
        .bind(self.id)
        .bind(status)
        .bind(by)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ticket)
    }

    /// Closes every ticket that has waited on its user for [`AUTO_CLOSE_AFTER_DAYS`].
    /// Returns how many were closed.
    pub async fn auto_close(db: &PgPool) -> sqlx::Result<u64> {
        let result = sqlx::query(
            "UPDATE support_tickets
                SET status = 'CLOSED', closed_at = now(), last_updated = now()
              WHERE status = 'AWAITING_USER'
                AND last_updated < now() - make_interval(days => $1)",
        )
        .bind(AUTO_CLOSE_AFTER_DAYS)
        .execute(db)
        .await?;
        Ok(result.rows_affected())
    }
}

async fn insert_message(
    conn: &mut PgConnection,
    ticket_id: i64,
    author_id: i64,
    content: &str,
    is_internal: bool,
    attachments: &[String],
) -> sqlx::Result<SupportMessage> {
    let mut message: SupportMessage = sqlx::query_as(concat!(
        "INSERT INTO support_messages (author_id, ticket_id, content, is_internal)
         VALUES ($1, $2, $3, $4)
         RETURNING ", message_columns!("support_messages")
    ))
    .bind(author_id)
    .bind(ticket_id)
    .bind(content)
    .bind(is_internal)
    .fetch_one(&mut *conn)
    .await?;
    message.attachments =
        Json(File::attach(conn, AttachTo::SupportMessage(message.id), author_id, attachments).await?);
    Ok(message)
}

async fn add_responder(conn: &mut PgConnection, ticket_id: i64, user_id: i64) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE support_tickets SET responders = array_append(responders, $2)
         WHERE id = $1 AND NOT ($2 = ANY(responders))",
    )
    .bind(ticket_id)
    .bind(user_id)
    .execute(conn)
    .await?;
    Ok(())
}

impl SupportMessage {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!(
            "SELECT ", message_columns!("m"), " FROM support_messages m WHERE m.id = $1"
        ))
        .bind(id)
        .fetch_optional(db)
        .await
    }

    /// Replaces the content, keeping the old content in `previous_content`. Callers check
    /// that the message hasn't been edited before.
    pub async fn edit(&self, db: &PgPool, content: &str) -> sqlx::Result<Self> {
        sqlx::query_as(concat!(
            "UPDATE support_messages
                SET previous_content = content, content = $2, last_updated = now()
              WHERE id = $1
              RETURNING ", message_columns!("support_messages")
        ))
        .bind(self.id)
        .bind(content)
        .fetch_one(db)
        .await
    }
}

impl Faq {
    /// Every FAQ, or those in `category`; most recently updated first.
    pub async fn list(db: &PgPool, category: Option<FaqCategory>, limit: Option<i64>) -> sqlx::Result<Vec<FaqWithPeople>> {
        sqlx::query_as(select_faq!(
            "WHERE ($1::faq_category IS NULL OR f.category = $1)
             ORDER BY f.last_updated DESC, f.id DESC
             LIMIT $2"
        ))
        .bind(category)
        .bind(limit)
        .fetch_all(db)
        .await
    }

    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<FaqWithPeople>> {
        sqlx::query_as(select_faq!("WHERE f.id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn create(db: &PgPool, created_by: i64, details: &FaqDetails) -> sqlx::Result<Self> {
        sqlx::query_as(concat!(
            "INSERT INTO faqs (title, subject, category, description, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING ", faq_columns!("faqs")
        ))
        .bind(&details.title)
        .bind(&details.subject)
        .bind(details.category)
        .bind(&details.description)
        .bind(created_by)
        .fetch_one(db)
        .await
    }

    /// Replaces the FAQ's details, recording `editor` if they haven't edited it before.
    /// `None` if there is no such FAQ.
    pub async fn update(db: &PgPool, id: i64, editor: i64, details: &FaqDetails) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!(
            "UPDATE faqs
                SET title = $2, subject = $3, category = $4, description = $5,
                    last_updated = now(),
                    editors = CASE WHEN $6 = ANY(editors) THEN editors
                                   ELSE array_append(editors, $6) END
              WHERE id = $1
              RETURNING ", faq_columns!("faqs")
        ))
        .bind(id)
        .bind(&details.title)
        .bind(&details.subject)
        .bind(details.category)
        .bind(&details.description)
        .bind(editor)
        .fetch_optional(db)
        .await
    }

    /// False if there was no such FAQ.
    pub async fn delete(db: &PgPool, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM faqs WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

impl UserRef {
    /// The users with these ids, in the order given. Ids with no user are skipped.
    pub async fn by_ids(db: &PgPool, ids: &[i64]) -> sqlx::Result<Vec<Self>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        sqlx::query_as(
            "SELECT u.id, u.username, u.avatar_url
             FROM unnest($1::BIGINT[]) WITH ORDINALITY AS ids(id, ord)
             JOIN users u ON u.id = ids.id
             ORDER BY ids.ord",
        )
        .bind(ids)
        .fetch_all(db)
        .await
    }
}
