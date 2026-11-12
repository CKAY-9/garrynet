//! Private messages between users; see docs/PRIVATE_MESSAGES.md. Users need Site Access
//! and User Messages, so suspended and banned users can't use them. A chat is addressed by
//! the other user's id. Staff with Admin Access and User Management can read anyone's
//! chats (read-only), and every such view is logged.

use actix_web::{HttpResponse, delete, get, post, web};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Usergroup,
        cursor::{Cursor, Page},
        private_message::{AdminMessage, Chat, NewMessage, PrivateMessage},
        support::UserRef,
    },
    notify,
    routes::{
        files,
        markdown::{local_images_only, markdown, plain_text},
        validate::{bad, present},
    },
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_chats)
        .service(unread_count)
        .service(get_thread)
        .service(send)
        .service(mark_read)
        .service(delete_chat)
        .service(admin_list_chats)
        .service(admin_get_thread);
}

/// Sending and reading messages; also what a recipient must have.
pub const SEND: Permissions = Permissions::SITE_ACCESS.union(Permissions::USER_MESSAGES);
/// Reading anyone's messages.
pub const VIEW_ANY: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::USER_MANAGEMENT);

const MAX_CONTENT: usize = 5_000;
/// Files per message.
const MAX_ATTACHMENTS: usize = 3;
/// At most this many messages per sender in any [`RATE_WINDOW_SECS`].
const RATE_LIMIT: i64 = 10;
const RATE_WINDOW_SECS: f64 = 30.0;
const DEFAULT_LIMIT: i64 = 25;
const MAX_LIMIT: i64 = 100;
/// Characters of a chat's last message shown in the chat list.
const MAX_PREVIEW: usize = 120;

/// Query string of the paged reads. Both optional; empty values are ignored.
#[derive(Deserialize)]
struct PageQuery {
    /// 1 to 100; 25 by default.
    limit: Option<String>,
    /// The previous page's `next_cursor`.
    cursor: Option<String>,
}

/// A parsed [`PageQuery`].
#[derive(Debug, PartialEq, Eq)]
struct PageParams {
    limit: i64,
    after: Option<Cursor>,
}

#[derive(Serialize)]
struct ChatList {
    #[serde(flatten)]
    page: Page<Chat>,
    /// Whether the user has ever sent a message; until they have, the message page shows
    /// the popup explaining that staff can read messages.
    has_sent: bool,
}

/// The user's chats, the most recent first: `{ items, next_cursor, has_sent }`.
#[get("/messages/chats")]
async fn list_chats(
    state: web::Data<AppState>,
    user: AuthUser,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SEND).await?;
    let params = parse_page(query.into_inner())?;
    let (page, has_sent) = tokio::try_join!(
        Chat::list_for(&state.db, user.0.id, params.after, params.limit),
        PrivateMessage::has_sent(&state.db, user.0.id),
    )?;
    Ok(HttpResponse::Ok().json(ChatList {
        page: with_previews(page),
        has_sent,
    }))
}

/// `{ count }`: unread messages the user can see, for the navigation badge.
#[get("/messages/unread-count")]
async fn unread_count(
    state: web::Data<AppState>,
    user: AuthUser,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SEND).await?;
    let count = PrivateMessage::unread_count(&state.db, user.0.id).await?;
    Ok(HttpResponse::Ok().json(json!({ "count": count })))
}

#[derive(Serialize)]
struct Thread<T> {
    /// The other user.
    user: UserRef,
    #[serde(flatten)]
    page: Page<T>,
}

/// The user's messages with another user, newest first: `{ user, items, next_cursor }`,
/// `user` being the other user. 404 if there is no such user.
#[get("/messages/with/{user_id}")]
async fn get_thread(
    state: web::Data<AppState>,
    user: AuthUser,
    other: web::Path<i64>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SEND).await?;
    let params = parse_page(query.into_inner())?;
    let other = find_user(&state, other.into_inner()).await?;
    let page =
        PrivateMessage::thread(&state.db, user.0.id, other.id, params.after, params.limit).await?;
    Ok(HttpResponse::Ok().json(Thread { user: other, page }))
}

#[derive(Deserialize)]
struct SendBody {
    /// Markdown.
    content: String,
    /// IDs of files of the user's to attach.
    #[serde(default)]
    attachments: Vec<String>,
    /// Hide the message from both users after 24 hours.
    #[serde(default)]
    temporary: bool,
}

/// Sends a message and returns it (201). The recipient must exist (404), not be the
/// sender (400) and have Site Access and User Messages themselves (a 403 that doesn't say
/// why). At most [`RATE_LIMIT`] per [`RATE_WINDOW_SECS`] (429). The recipient is notified
/// once the message is stored.
#[post("/messages/with/{user_id}")]
async fn send(
    state: web::Data<AppState>,
    user: AuthUser,
    recipient: web::Path<i64>,
    body: web::Json<SendBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SEND).await?;
    let recipient_id = recipient.into_inner();
    let mut new = normalize_send(body.into_inner(), user.0.id, recipient_id)?;
    local_images_only(
        "content",
        &new.content,
        &format!("{}/file/", state.config.backend_url),
    )?;

    let recipient = find_user(&state, recipient_id).await?;
    if !Usergroup::permissions_for_user(&state.db, recipient.id)
        .await?
        .contains(SEND)
    {
        return Err(ApiError::Forbidden(
            "a recipient who can receive messages".to_owned(),
        ));
    }
    let (count, oldest) =
        PrivateMessage::recent_sent_by(&state.db, user.0.id, RATE_WINDOW_SECS).await?;
    if count >= RATE_LIMIT {
        let elapsed = oldest.map_or(0.0, |t| (Utc::now() - t).num_milliseconds() as f64 / 1000.0);
        return Err(ApiError::RateLimited(
            (RATE_WINDOW_SECS - elapsed).ceil().max(1.0) as u64,
        ));
    }
    new.attachments =
        files::attachable(&state, user.0.id, &new.attachments, MAX_ATTACHMENTS).await?;

    let message = PrivateMessage::insert(&state.db, user.0.id, recipient.id, &new).await?;
    notify::private_message(&state.db, &message, &user.0).await;
    Ok(HttpResponse::Created().json(message))
}

/// Marks what the other user sent the user as read. Idempotent; returns `{ updated }`, how
/// many were unread.
#[post("/messages/with/{user_id}/read")]
async fn mark_read(
    state: web::Data<AppState>,
    user: AuthUser,
    other: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SEND).await?;
    let updated = PrivateMessage::mark_read(&state.db, user.0.id, other.into_inner()).await?;
    Ok(HttpResponse::Ok().json(json!({ "updated": updated })))
}

/// Deletes the chat with another user for the user only (204). The other user's view
/// doesn't change and they aren't told.
#[delete("/messages/with/{user_id}")]
async fn delete_chat(
    state: web::Data<AppState>,
    user: AuthUser,
    other: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SEND).await?;
    PrivateMessage::delete_chat(&state.db, user.0.id, other.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize)]
struct AdminChatList {
    /// Whose chats these are.
    user: UserRef,
    #[serde(flatten)]
    page: Page<Chat>,
}

/// Staff: a user's chats, including ones they deleted or whose messages expired:
/// `{ user, items, next_cursor }`. Logged.
#[get("/messages/admin/{user_id}/chats")]
async fn admin_list_chats(
    state: web::Data<AppState>,
    staff: AuthUser,
    user_id: web::Path<i64>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    staff.require(&state.db, VIEW_ANY).await?;
    let params = parse_page(query.into_inner())?;
    let user = find_user(&state, user_id.into_inner()).await?;
    log::info!(
        "user {} viewed the private message chats of user {}",
        staff.0.id,
        user.id
    );
    let page = Chat::admin_list_for(&state.db, user.id, params.after, params.limit).await?;
    Ok(HttpResponse::Ok().json(AdminChatList {
        user,
        page: with_previews(page),
    }))
}

#[derive(Serialize)]
struct AdminThread {
    /// The user whose chats are being read.
    user: UserRef,
    /// The other user in the chat.
    other: UserRef,
    #[serde(flatten)]
    page: Page<AdminMessage>,
}

/// Staff: every message between two users, newest first, deleted and expired ones
/// included and flagged: `{ user, other, items, next_cursor }`. Logged.
#[get("/messages/admin/{user_id}/with/{other_id}")]
async fn admin_get_thread(
    state: web::Data<AppState>,
    staff: AuthUser,
    path: web::Path<(i64, i64)>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    staff.require(&state.db, VIEW_ANY).await?;
    let params = parse_page(query.into_inner())?;
    let (user_id, other_id) = path.into_inner();
    let (user, other) = tokio::try_join!(find_user(&state, user_id), find_user(&state, other_id))?;
    log::info!(
        "user {} viewed the private messages between users {} and {}",
        staff.0.id,
        user.id,
        other.id
    );
    let page =
        PrivateMessage::admin_thread(&state.db, user.id, other.id, params.after, params.limit)
            .await?;
    Ok(HttpResponse::Ok().json(AdminThread { user, other, page }))
}

/// 404 unless the user exists.
async fn find_user(state: &AppState, id: i64) -> Result<UserRef, ApiError> {
    UserRef::by_ids(&state.db, &[id])
        .await?
        .into_iter()
        .next()
        .ok_or(ApiError::NotFound("user"))
}

/// Turns each chat's last message into a short plain-text preview.
fn with_previews(mut page: Page<Chat>) -> Page<Chat> {
    for chat in &mut page.items {
        chat.preview = preview(&chat.preview);
    }
    page
}

fn preview(content: &str) -> String {
    notify::clip(&plain_text(content), MAX_PREVIEW)
}

fn parse_page(query: PageQuery) -> Result<PageParams, ApiError> {
    let limit = match present(query.limit) {
        None => DEFAULT_LIMIT,
        Some(v) => v
            .parse::<i64>()
            .ok()
            .filter(|l| (1..=MAX_LIMIT).contains(l))
            .ok_or_else(|| bad(format!("limit must be between 1 and {MAX_LIMIT}")))?,
    };
    let after = present(query.cursor)
        .map(|v| {
            Cursor::parse(&v).ok_or_else(|| bad("cursor is not a next_cursor value".to_owned()))
        })
        .transpose()?;
    Ok(PageParams { limit, after })
}

/// Checks what can be checked without the database: not to oneself (400), content of 1 to
/// [`MAX_CONTENT`] characters, sanitized, and at most [`MAX_ATTACHMENTS`] distinct files.
/// The route checks the rest (local images, the recipient, the files themselves).
fn normalize_send(
    body: SendBody,
    sender_id: i64,
    recipient_id: i64,
) -> Result<NewMessage, ApiError> {
    if sender_id == recipient_id {
        return Err(bad("you can't send a message to yourself".to_owned()));
    }
    let content = markdown("content", &body.content, MAX_CONTENT)?;
    let mut attachments: Vec<String> = Vec::with_capacity(body.attachments.len());
    for id in body.attachments {
        if !attachments.contains(&id) {
            attachments.push(id);
        }
    }
    if attachments.len() > MAX_ATTACHMENTS {
        return Err(bad(format!(
            "at most {MAX_ATTACHMENTS} files can be attached"
        )));
    }
    Ok(NewMessage {
        content,
        attachments,
        temporary: body.temporary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(content: &str, attachments: &[&str]) -> SendBody {
        SendBody {
            content: content.into(),
            attachments: attachments.iter().map(|&s| s.to_owned()).collect(),
            temporary: false,
        }
    }

    #[test]
    fn send_bodies() {
        let new = normalize_send(body("  hello <b>there</b> ", &["a", "b", "a"]), 1, 2)
            .unwrap_or_else(|_| panic!("valid body rejected"));
        assert_eq!(
            new.content, "hello \\<b>there\\</b>",
            "trimmed, HTML escaped"
        );
        assert_eq!(new.attachments, vec!["a", "b"], "duplicates dropped");
        assert!(!new.temporary);

        let temporary = SendBody {
            temporary: true,
            ..body("hi", &[])
        };
        assert!(normalize_send(temporary, 1, 2).is_ok_and(|n| n.temporary));

        assert!(
            normalize_send(body("hi", &[]), 7, 7).is_err(),
            "to yourself"
        );
        assert!(normalize_send(body("", &[]), 1, 2).is_err());
        assert!(
            normalize_send(body("  \n ", &["a"]), 1, 2).is_err(),
            "attachments don't replace content"
        );
        assert!(normalize_send(body(&"x".repeat(MAX_CONTENT), &[]), 1, 2).is_ok());
        assert!(normalize_send(body(&"x".repeat(MAX_CONTENT + 1), &[]), 1, 2).is_err());
        assert!(normalize_send(body("hi", &["a", "b", "c"]), 1, 2).is_ok());
        assert!(normalize_send(body("hi", &["a", "b", "c", "d"]), 1, 2).is_err());
        assert!(normalize_send(body("[x](javascript:alert(1))", &[]), 1, 2).is_err());
    }

    fn query(limit: &str, cursor: &str) -> PageQuery {
        PageQuery {
            limit: Some(limit.into()),
            cursor: Some(cursor.into()),
        }
    }

    #[test]
    fn pages() {
        let params = parse_page(query("", "")).unwrap();
        assert_eq!(
            params,
            PageParams {
                limit: DEFAULT_LIMIT,
                after: None
            }
        );
        let params = parse_page(PageQuery {
            limit: None,
            cursor: None,
        })
        .unwrap();
        assert_eq!(params.limit, DEFAULT_LIMIT);

        let params = parse_page(query(" 100 ", "1794000000123456-42")).unwrap();
        assert_eq!(params.limit, 100);
        assert_eq!(params.after, Cursor::parse("1794000000123456-42"));
        assert_eq!(parse_page(query("1", "")).unwrap().limit, 1);

        assert!(parse_page(query("0", "")).is_err());
        assert!(parse_page(query("101", "")).is_err());
        assert!(parse_page(query("-5", "")).is_err());
        assert!(parse_page(query("ten", "")).is_err());
        assert!(parse_page(query("", "nope")).is_err());
        assert!(parse_page(query("", "123")).is_err());
    }

    #[test]
    fn previews() {
        assert_eq!(
            preview("**hey**, look at\n\n![cat](https://x/c.png)"),
            "hey, look at cat"
        );
        let long = preview(&"word ".repeat(100));
        assert_eq!(long.chars().count(), MAX_PREVIEW);
        assert!(long.ends_with('…'));
    }

    #[test]
    fn permission_masks() {
        assert!(Permissions::BASIC.contains(SEND), "every user can message");
        assert!(!Permissions::BASIC.contains(VIEW_ANY));
    }
}
