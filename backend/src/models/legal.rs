//! The community's legal documents (docs/LEGAL.md): terms of service, privacy policy and
//! refund policy, as Markdown. Each deployment edits its own; GarryNet ships defaults.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;

/// A document GarryNet ships a default for. The slug is its URL (`/legal/{slug}`).
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Template {
    pub slug: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

/// Every legal document, in the order they're listed. The migration's slug CHECK must
/// match.
pub const TEMPLATES: [Template; 3] = [
    Template {
        slug: "terms",
        title: "Terms of Service",
        body: include_str!("../../legal/terms.md"),
    },
    Template {
        slug: "privacy",
        title: "Privacy Policy",
        body: include_str!("../../legal/privacy.md"),
    },
    Template {
        slug: "refunds",
        title: "Refund Policy",
        body: include_str!("../../legal/refunds.md"),
    },
];

/// The default for `slug`, or `None` if there's no such document.
pub fn template(slug: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.slug == slug)
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct LegalDocument {
    pub slug: String,
    pub title: String,
    pub body: String,
    /// `None` while it's still the default template.
    pub last_updated: Option<DateTime<Utc>>,
    pub updated_by: Option<i64>,
}

/// A document without its body, for listing.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct LegalSummary {
    pub slug: String,
    pub title: String,
    pub last_updated: Option<DateTime<Utc>>,
}

impl LegalDocument {
    pub async fn get(db: &PgPool, slug: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "SELECT slug, title, body, last_updated, updated_by FROM legal_documents WHERE slug = $1",
        )
        .bind(slug)
        .fetch_optional(db)
        .await
    }

    /// Every document, in [`TEMPLATES`] order.
    pub async fn list(db: &PgPool) -> sqlx::Result<Vec<LegalSummary>> {
        let order: Vec<&str> = TEMPLATES.iter().map(|t| t.slug).collect();
        sqlx::query_as(
            "SELECT slug, title, last_updated FROM legal_documents
             ORDER BY array_position($1::text[], slug)",
        )
        .bind(order)
        .fetch_all(db)
        .await
    }

    /// Replaces a document's title and body (already validated), or `None` if it doesn't exist.
    pub async fn update(
        db: &PgPool,
        slug: &str,
        title: &str,
        body: &str,
        user_id: i64,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "UPDATE legal_documents
             SET title = $2, body = $3, last_updated = now(), updated_by = $4
             WHERE slug = $1
             RETURNING slug, title, body, last_updated, updated_by",
        )
        .bind(slug)
        .bind(title)
        .bind(body)
        .bind(user_id)
        .fetch_optional(db)
        .await
    }

    /// Inserts the default template of every document that doesn't exist yet. Existing
    /// documents are left untouched, so admins' edits survive restarts and upgrades.
    pub async fn ensure_seeded(db: &PgPool) -> sqlx::Result<()> {
        for t in &TEMPLATES {
            let inserted = sqlx::query(
                "INSERT INTO legal_documents (slug, title, body) VALUES ($1, $2, $3)
                 ON CONFLICT (slug) DO NOTHING",
            )
            .bind(t.slug)
            .bind(t.title)
            // Trimmed, as saving through the API would store it.
            .bind(t.body.trim())
            .execute(db)
            .await?
            .rows_affected();
            if inserted > 0 {
                log::info!("created the default {}; edit it at /admin/legal", t.title);
            }
        }
        Ok(())
    }
}
