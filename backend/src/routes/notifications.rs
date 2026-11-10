//! A user's notifications; see docs/NOTIFICATIONS.md. Login is required but Site Access
//! isn't: banned users can still use support and need to hear back about their tickets.
//! Users only ever see and change their own; anyone else's id answers 404. Notifications
//! are created by `crate::notify`, never through a route.

use actix_web::{HttpResponse, delete, get, post, web};
use serde::Deserialize;
use serde_json::json;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::notification::{Cursor, Notification, NotificationFilter, NotificationType},
    routes::validate::{bad, parse_enum, present},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list)
        .service(unread_count)
        .service(mark_all_read)
        .service(mark_read)
        .service(delete_read)
        .service(delete_one);
}

const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;

/// Query string of the list. Every field is optional, and empty values are ignored.
#[derive(Deserialize)]
struct ListQuery {
    /// `true` for unread notifications only.
    unread: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    /// 1 to 100; 25 by default.
    limit: Option<String>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
}

/// The user's notifications, newest first: `{ notifications, next_cursor }`, where
/// `next_cursor` is `null` on the last page.
#[get("/notifications")]
async fn list(
    state: web::Data<AppState>,
    user: AuthUser,
    query: web::Query<ListQuery>,
) -> Result<HttpResponse, ApiError> {
    let filter = parse_filter(query.into_inner())?;
    let page = Notification::list(&state.db, user.0.id, &filter).await?;
    Ok(HttpResponse::Ok().json(page))
}

/// `{ count }`: how many of the user's notifications are unread, for the navigation badge.
#[get("/notifications/unread-count")]
async fn unread_count(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    let count = Notification::unread_count(&state.db, user.0.id).await?;
    Ok(HttpResponse::Ok().json(json!({ "count": count })))
}

/// Marks one notification read and returns it. Idempotent: `read_at` keeps the time it was
/// first read.
#[post("/notifications/{id}/read")]
async fn mark_read(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let notification = Notification::mark_read(&state.db, user.0.id, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("notification"))?;
    Ok(HttpResponse::Ok().json(notification))
}

#[derive(Default, Deserialize)]
struct ReadAllBody {
    /// Only notifications of this type.
    #[serde(default, rename = "type")]
    kind: Option<NotificationType>,
}

/// Marks all of the user's notifications read, or only those of the body's `type`. The
/// body is optional. Returns `{ updated }`, how many were unread.
#[post("/notifications/read-all")]
async fn mark_all_read(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Bytes,
) -> Result<HttpResponse, ApiError> {
    let body = parse_read_all(&body)?;
    let updated = Notification::mark_all_read(&state.db, user.0.id, body.kind).await?;
    Ok(HttpResponse::Ok().json(json!({ "updated": updated })))
}

/// Deletes one of the user's notifications (204).
#[delete("/notifications/{id}")]
async fn delete_one(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    if !Notification::delete(&state.db, user.0.id, id.into_inner()).await? {
        return Err(ApiError::NotFound("notification"));
    }
    Ok(HttpResponse::NoContent().finish())
}

/// Deletes all of the user's read notifications. Returns `{ deleted }`.
#[delete("/notifications")]
async fn delete_read(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    let deleted = Notification::delete_read(&state.db, user.0.id).await?;
    Ok(HttpResponse::Ok().json(json!({ "deleted": deleted })))
}

fn parse_filter(query: ListQuery) -> Result<NotificationFilter, ApiError> {
    let unread_only = match present(query.unread).as_deref() {
        None | Some("false") => false,
        Some("true") => true,
        Some(_) => return Err(bad("unread must be true or false".to_owned())),
    };
    let limit = match present(query.limit) {
        None => DEFAULT_LIMIT,
        Some(v) => v
            .parse::<i64>()
            .ok()
            .filter(|l| (1..=MAX_LIMIT).contains(l))
            .ok_or_else(|| bad(format!("limit must be between 1 and {MAX_LIMIT}")))?,
    };
    Ok(NotificationFilter {
        unread_only,
        kind: present(query.kind)
            .map(|v| parse_enum::<NotificationType>("type", &v))
            .transpose()?,
        limit,
        after: present(query.cursor)
            .map(|v| Cursor::parse(&v).ok_or_else(|| bad("cursor is not a next_cursor value".to_owned())))
            .transpose()?,
    })
}

/// An empty body means every type.
fn parse_read_all(body: &[u8]) -> Result<ReadAllBody, ApiError> {
    if body.trim_ascii().is_empty() {
        return Ok(ReadAllBody::default());
    }
    serde_json::from_slice(body)
        .map_err(|_| bad("body must be empty or JSON like {\"type\": \"STORE\"}".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query(unread: &str, kind: &str, limit: &str, cursor: &str) -> ListQuery {
        ListQuery {
            unread: Some(unread.into()),
            kind: Some(kind.into()),
            limit: Some(limit.into()),
            cursor: Some(cursor.into()),
        }
    }

    #[test]
    fn filters() {
        let filter = parse_filter(query("", "", "", "")).unwrap();
        assert!(!filter.unread_only);
        assert_eq!(filter.kind, None);
        assert_eq!(filter.limit, DEFAULT_LIMIT);
        assert_eq!(filter.after, None);

        let filter = parse_filter(query("true", "store", "100", "1794000000123456-42")).unwrap();
        assert!(filter.unread_only);
        assert_eq!(filter.kind, Some(NotificationType::Store));
        assert_eq!(filter.limit, 100);
        assert!(filter.after.is_some());

        assert!(parse_filter(query("yes", "", "", "")).is_err());
        assert!(parse_filter(query("", "MENTION", "", "")).is_err());
        assert!(parse_filter(query("", "", "0", "")).is_err());
        assert!(parse_filter(query("", "", "101", "")).is_err());
        assert!(parse_filter(query("", "", "", "nope")).is_err());
    }

    #[test]
    fn read_all_bodies() {
        assert_eq!(parse_read_all(b"").unwrap().kind, None);
        assert_eq!(parse_read_all(b" {} ").unwrap().kind, None);
        assert_eq!(parse_read_all(br#"{"type": "FORUM"}"#).unwrap().kind, Some(NotificationType::Forum));
        assert!(parse_read_all(br#"{"type": "forum"}"#).is_err());
        assert!(parse_read_all(b"type=FORUM").is_err());
    }
}
