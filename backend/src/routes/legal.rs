//! The community's legal documents (docs/LEGAL.md). Anyone can read them; Community
//! Management edits them.

use actix_web::{HttpResponse, get, put, web};
use serde::Deserialize;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions,
        legal::{self, LegalDocument},
    },
    routes::{markdown::markdown, validate::required_text},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_documents)
        .service(get_document)
        .service(get_template)
        .service(update_document);
}

/// Every document's slug, title and last update. Public: the navigation links to them.
#[get("/legal")]
async fn list_documents(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(LegalDocument::list(&state.db).await?))
}

/// A document in full. Public.
#[get("/legal/{slug}")]
async fn get_document(
    state: web::Data<AppState>,
    slug: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let document = LegalDocument::get(&state.db, &slug)
        .await?
        .ok_or(ApiError::NotFound("legal document"))?;
    Ok(HttpResponse::Ok().json(document))
}

/// The default template GarryNet ships for a document, to reset it. Requires Admin Access
/// and Community Management.
#[get("/legal/{slug}/template")]
async fn get_template(
    state: web::Data<AppState>,
    user: AuthUser,
    slug: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let template = legal::template(&slug).ok_or(ApiError::NotFound("legal document"))?;
    Ok(HttpResponse::Ok().json(template))
}

/// Replaces a document's title and body. Requires Admin Access and Community Management.
#[put("/legal/{slug}")]
async fn update_document(
    state: web::Data<AppState>,
    user: AuthUser,
    slug: web::Path<String>,
    body: web::Json<DocumentBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let (title, text) = validate(&body)?;
    let document = LegalDocument::update(&state.db, &slug, &title, &text, user.0.id)
        .await?
        .ok_or(ApiError::NotFound("legal document"))?;
    log::info!("user {} updated the {}", user.0.id, document.slug);
    Ok(HttpResponse::Ok().json(document))
}

/// Like the other admin pages, editing needs Admin Access as well as the specific permission.
const MANAGE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::COMMUNITY_MANAGEMENT);

#[derive(Deserialize)]
struct DocumentBody {
    title: String,
    body: String,
}

const MAX_TITLE: usize = 100;
const MAX_BODY: usize = 100_000;

/// The trimmed title and the sanitized Markdown body.
fn validate(body: &DocumentBody) -> Result<(String, String), ApiError> {
    Ok((
        required_text("title", &body.title, MAX_TITLE)?,
        markdown("body", &body.body, MAX_BODY)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(title: &str, text: &str) -> DocumentBody {
        DocumentBody { title: title.into(), body: text.into() }
    }

    #[test]
    fn templates_are_valid_and_unchanged_by_validation() {
        for t in &legal::TEMPLATES {
            let (title, text) = validate(&body(t.title, t.body))
                .unwrap_or_else(|e| panic!("{} template rejected: {e}", t.slug));
            assert_eq!(title, t.title);
            // No raw HTML for the sanitizer to escape: the seeded text is what an admin
            // would get by saving it unchanged.
            assert_eq!(text, t.body.trim(), "{} template changed by validation", t.slug);
        }
    }

    #[test]
    fn rejects_bad_input() {
        assert!(validate(&body(" ", "text")).is_err());
        assert!(validate(&body("Terms", "  ")).is_err());
        assert!(validate(&body("Terms", "[x](javascript:alert(1))")).is_err());
        assert!(validate(&body("Terms", &"a".repeat(MAX_BODY + 1))).is_err());
    }

    #[test]
    fn escapes_html() {
        let (_, text) = validate(&body("Terms", "<script>alert(1)</script>")).unwrap();
        // Backslash-escaped, so it renders as text.
        assert!(text.starts_with("\\<script"));
    }
}
