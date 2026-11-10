//! The forum (docs/FORUM.md): sections hold categories, categories hold threads, threads
//! hold posts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Json};

use super::file::{AttachTo, Attachment, File, attachments_json};

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ForumSection {
    pub id: i64,
    pub name: String,
    /// Hex color.
    pub accent: String,
    pub icon_url: Option<String>,
    pub description: String,
    /// Listed highest to lowest; -1 is always last.
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ForumCategory {
    pub id: i64,
    pub name: String,
    pub section_id: i64,
    pub description: String,
    /// Prevents new threads and posts.
    pub locked: bool,
    pub position: i32,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ForumThread {
    pub id: i64,
    pub author_id: i64,
    pub category_id: i64,
    pub title: String,
    pub thread_post_id: i64,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub locked: bool,
    pub sticky: bool,
    pub allow_template: bool,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ForumPost {
    pub id: i64,
    pub author_id: i64,
    pub thread_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited: Option<DateTime<Utc>>,
    pub likes: Vec<i64>,
    pub dislikes: Vec<i64>,
    /// Files attached when the post was written.
    pub attachments: Json<Vec<Attachment>>,
}

/// The parts of a user the forum displays next to their threads and posts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForumAuthor {
    pub id: i64,
    pub username: String,
    pub avatar_url: String,
}

/// The most recently active thread in a category, for the forum homepage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatestThread {
    pub id: i64,
    pub title: String,
    pub last_activity: DateTime<Utc>,
    pub author: ForumAuthor,
}

/// A category with its activity, as listed on the forum homepage.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct CategorySummary {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub category: ForumCategory,
    pub thread_count: i64,
    pub post_count: i64,
    pub latest: Option<Json<LatestThread>>,
}

/// A section with its categories, in display order.
#[derive(Clone, Debug, Serialize)]
pub struct SectionWithCategories {
    #[serde(flatten)]
    pub section: ForumSection,
    pub categories: Vec<CategorySummary>,
}

/// A thread as listed in its category.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ThreadSummary {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub thread: ForumThread,
    pub author: Json<ForumAuthor>,
    /// Posts other than the thread's first.
    pub replies: i64,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct PostWithAuthor {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub post: ForumPost,
    pub author: Json<ForumAuthor>,
}

/// Already validated (see `routes::forum`).
pub struct SectionDetails {
    pub name: String,
    pub accent: String,
    pub icon_url: Option<String>,
    pub description: String,
    pub position: i32,
}

/// Already validated (see `routes::forum`).
pub struct CategoryDetails {
    pub name: String,
    pub section_id: i64,
    pub description: String,
    pub locked: bool,
    pub position: i32,
}

// SQL fragments, as macros so queries stay `&'static str` (via `concat!`).

/// Highest position first, -1 always last, then oldest first.
macro_rules! order_by_position {
    () => {
        "(position = -1), position DESC, id"
    };
}

/// A JSON object of the user aliased `u`, decoded as a [`ForumAuthor`].
macro_rules! author_json {
    () => {
        "json_build_object('id', u.id, 'username', u.username, 'avatar_url', u.avatar_url)"
    };
}

macro_rules! section_columns {
    () => {
        "id, name, accent, icon_url, description, position, created_at, last_updated"
    };
}

macro_rules! category_columns {
    () => {
        "id, name, section_id, description, locked, position, created_at, last_updated"
    };
}

macro_rules! thread_columns {
    () => {
        "id, author_id, category_id, title, thread_post_id, created_at, last_activity, locked,
         sticky, allow_template"
    };
    (t) => {
        "t.id, t.author_id, t.category_id, t.title, t.thread_post_id, t.created_at,
         t.last_activity, t.locked, t.sticky, t.allow_template"
    };
}

macro_rules! post_columns {
    () => {
        concat!(
            "id, author_id, thread_id, content, created_at, edited, likes, dislikes, ",
            attachments_json!("forum_post_id", "forum_posts.id"), " AS attachments"
        )
    };
    (p) => {
        concat!(
            "p.id, p.author_id, p.thread_id, p.content, p.created_at, p.edited, p.likes, p.dislikes, ",
            attachments_json!("forum_post_id", "p.id"), " AS attachments"
        )
    };
}

/// Forum-wide totals for the forum homepage.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ForumStats {
    pub threads: i64,
    pub posts: i64,
}

impl ForumSection {
    /// Every section with its categories, both in display order.
    pub async fn list_with_categories(db: &PgPool) -> sqlx::Result<Vec<SectionWithCategories>> {
        let sections: Vec<ForumSection> = sqlx::query_as(concat!(
            "SELECT ", section_columns!(), " FROM forum_sections ORDER BY ", order_by_position!()
        ))
        .fetch_all(db)
        .await?;

        let categories: Vec<CategorySummary> = sqlx::query_as(concat!(
            "SELECT ", category_columns!(), ",
                 (SELECT count(*) FROM forum_threads t WHERE t.category_id = c.id) AS thread_count,
                 (SELECT count(*) FROM forum_posts p JOIN forum_threads t ON t.id = p.thread_id
                  WHERE t.category_id = c.id) AS post_count,
                 (SELECT json_build_object('id', t.id, 'title', t.title,
                                           'last_activity', t.last_activity, 'author', ", author_json!(), ")
                  FROM forum_threads t JOIN users u ON u.id = t.author_id
                  WHERE t.category_id = c.id
                  ORDER BY t.last_activity DESC LIMIT 1) AS latest
             FROM forum_categories c
             ORDER BY ", order_by_position!()
        ))
        .fetch_all(db)
        .await?;

        let mut result: Vec<SectionWithCategories> = sections
            .into_iter()
            .map(|section| SectionWithCategories { section, categories: Vec::new() })
            .collect();
        for category in categories {
            if let Some(s) = result.iter_mut().find(|s| s.section.id == category.category.section_id) {
                s.categories.push(category);
            }
        }
        Ok(result)
    }

    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!("SELECT ", section_columns!(), " FROM forum_sections WHERE id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn create(db: &PgPool, details: &SectionDetails) -> sqlx::Result<Self> {
        sqlx::query_as(concat!(
            "INSERT INTO forum_sections (name, accent, icon_url, description, position)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING ", section_columns!()
        ))
        .bind(&details.name)
        .bind(&details.accent)
        .bind(&details.icon_url)
        .bind(&details.description)
        .bind(details.position)
        .fetch_one(db)
        .await
    }

    /// Deletes the section and everything in it. False if there was no such section.
    pub async fn delete(db: &PgPool, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM forum_sections WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

impl ForumCategory {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!("SELECT ", category_columns!(), " FROM forum_categories WHERE id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn create(db: &PgPool, details: &CategoryDetails) -> sqlx::Result<Self> {
        sqlx::query_as(concat!(
            "INSERT INTO forum_categories (name, section_id, description, locked, position)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING ", category_columns!()
        ))
        .bind(&details.name)
        .bind(details.section_id)
        .bind(&details.description)
        .bind(details.locked)
        .bind(details.position)
        .fetch_one(db)
        .await
    }

    /// Deletes the category and its threads. False if there was no such category.
    pub async fn delete(db: &PgPool, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM forum_categories WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// The category's threads: stickies first, then by most recent activity.
    pub async fn threads(db: &PgPool, id: i64) -> sqlx::Result<Vec<ThreadSummary>> {
        sqlx::query_as(concat!(
            "SELECT ", thread_columns!(t), ", ", author_json!(), " AS author,
                 (SELECT count(*) FROM forum_posts p WHERE p.thread_id = t.id) - 1 AS replies
             FROM forum_threads t JOIN users u ON u.id = t.author_id
             WHERE t.category_id = $1
             ORDER BY t.sticky DESC, t.last_activity DESC, t.id DESC"
        ))
        .bind(id)
        .fetch_all(db)
        .await
    }
}

impl ForumThread {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!("SELECT ", thread_columns!(), " FROM forum_threads WHERE id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// Creates a thread and its first post together.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        db: &PgPool,
        author_id: i64,
        category_id: i64,
        title: &str,
        content: &str,
        locked: bool,
        allow_template: bool,
        attachments: &[String],
    ) -> sqlx::Result<Self> {
        let mut tx = db.begin().await?;
        // The thread references its first post (a deferred foreign key), so reserve the
        // post's id before inserting either.
        let post_id: i64 =
            sqlx::query_scalar("SELECT nextval(pg_get_serial_sequence('forum_posts', 'id'))")
                .fetch_one(&mut *tx)
                .await?;
        let thread: Self = sqlx::query_as(concat!(
            "INSERT INTO forum_threads (author_id, category_id, title, thread_post_id, locked, allow_template)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING ", thread_columns!()
        ))
        .bind(author_id)
        .bind(category_id)
        .bind(title)
        .bind(post_id)
        .bind(locked)
        .bind(allow_template)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO forum_posts (id, author_id, thread_id, content, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(post_id)
        .bind(author_id)
        .bind(thread.id)
        .bind(content)
        .bind(thread.created_at)
        .execute(&mut *tx)
        .await?;
        File::attach(&mut tx, AttachTo::ForumPost(post_id), author_id, attachments).await?;
        tx.commit().await?;
        Ok(thread)
    }

    /// Every post: the thread's first post, then replies oldest first.
    pub async fn posts(&self, db: &PgPool) -> sqlx::Result<Vec<PostWithAuthor>> {
        sqlx::query_as(concat!(
            "SELECT ", post_columns!(p), ", ", author_json!(), " AS author
             FROM forum_posts p JOIN users u ON u.id = p.author_id
             WHERE p.thread_id = $1
             ORDER BY (p.id = $2) DESC, p.created_at, p.id"
        ))
        .bind(self.id)
        .bind(self.thread_post_id)
        .fetch_all(db)
        .await
    }

    /// Adds a reply with its attachments (files of the author's; see `File::attach`) and
    /// bumps the thread's last activity.
    pub async fn reply(
        &self,
        db: &PgPool,
        author_id: i64,
        content: &str,
        attachments: &[String],
    ) -> sqlx::Result<ForumPost> {
        let mut tx = db.begin().await?;
        let mut post: ForumPost = sqlx::query_as(concat!(
            "INSERT INTO forum_posts (author_id, thread_id, content)
             VALUES ($1, $2, $3)
             RETURNING ", post_columns!()
        ))
        .bind(author_id)
        .bind(self.id)
        .bind(content)
        .fetch_one(&mut *tx)
        .await?;
        post.attachments = Json(File::attach(&mut tx, AttachTo::ForumPost(post.id), author_id, attachments).await?);
        sqlx::query("UPDATE forum_threads SET last_activity = $2 WHERE id = $1")
            .bind(self.id)
            .bind(post.created_at)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(post)
    }

    /// Applies the changes that are set. Changing a thread doesn't count as activity.
    pub async fn update(&self, db: &PgPool, changes: &ThreadChanges) -> sqlx::Result<Self> {
        sqlx::query_as(concat!(
            "UPDATE forum_threads
                SET title = COALESCE($2, title),
                    locked = COALESCE($3, locked),
                    sticky = COALESCE($4, sticky),
                    allow_template = COALESCE($5, allow_template)
              WHERE id = $1
              RETURNING ", thread_columns!()
        ))
        .bind(self.id)
        .bind(&changes.title)
        .bind(changes.locked)
        .bind(changes.sticky)
        .bind(changes.allow_template)
        .fetch_one(db)
        .await
    }

    /// Deletes the thread and all its posts.
    pub async fn delete(&self, db: &PgPool) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM forum_threads WHERE id = $1")
            .bind(self.id)
            .execute(db)
            .await?;
        Ok(())
    }
}

/// Thread fields to change; `None` leaves a field as it is. Already validated (see
/// `routes::forum`).
#[derive(Debug, Default)]
pub struct ThreadChanges {
    pub title: Option<String>,
    pub locked: Option<bool>,
    pub sticky: Option<bool>,
    pub allow_template: Option<bool>,
}

impl ForumPost {
    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(concat!("SELECT ", post_columns!(), " FROM forum_posts WHERE id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// Replaces the content and marks the post edited.
    pub async fn edit(&self, db: &PgPool, content: &str) -> sqlx::Result<Self> {
        sqlx::query_as(concat!(
            "UPDATE forum_posts SET content = $2, edited = now()
             WHERE id = $1
             RETURNING ", post_columns!()
        ))
        .bind(self.id)
        .bind(content)
        .fetch_one(db)
        .await
    }

    /// Deletes a reply and rewinds the thread's last activity to its newest remaining post.
    /// A thread's first post is deleted with [`ForumThread::delete`] instead.
    pub async fn delete_reply(&self, db: &PgPool) -> sqlx::Result<()> {
        let mut tx = db.begin().await?;
        sqlx::query("DELETE FROM forum_posts WHERE id = $1")
            .bind(self.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE forum_threads t
                SET last_activity = COALESCE(
                    (SELECT max(p.created_at) FROM forum_posts p WHERE p.thread_id = t.id),
                    t.created_at)
              WHERE t.id = $1",
        )
        .bind(self.thread_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

impl ForumStats {
    pub async fn get(db: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as(
            "SELECT (SELECT count(*) FROM forum_threads) AS threads,
                    (SELECT count(*) FROM forum_posts) AS posts",
        )
        .fetch_one(db)
        .await
    }

    /// Posts made in the past `days` days, for the admin dashboard.
    pub async fn posts_since_days(db: &PgPool, days: i32) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "SELECT count(*) FROM forum_posts WHERE created_at > now() - make_interval(days => $1)",
        )
        .bind(days)
        .fetch_one(db)
        .await
    }
}
