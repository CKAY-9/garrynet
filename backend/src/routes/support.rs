//! The support system; see docs/SUPPORT.MD. Any logged-in user can open tickets and reply
//! to their own (Site Access isn't needed: banned users may still use support). The
//! Support Team sees every ticket, replies to any, sets statuses and manages FAQs, which
//! anyone can read.

use actix_web::{HttpResponse, delete, get, patch, post, put, web};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, User,
        support::{
            Faq, FaqCategory, FaqDetails, FaqWithPeople, MAX_ACTIVE_TICKETS, MessageWithAuthor,
            SupportMessage, SupportStatus, SupportTicket, TicketCategory, TicketDetails,
            TicketFilter, TicketSort, UserRef,
        },
    },
    routes::{
        files,
        markdown::markdown,
        validate::{bad, required_text},
    },
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_my_tickets)
        .service(create_ticket)
        .service(list_all_tickets)
        .service(get_ticket)
        .service(create_message)
        .service(set_status)
        .service(edit_message)
        .service(list_faqs)
        .service(get_faq)
        .service(create_faq)
        .service(update_faq)
        .service(delete_faq);
}

const SUPPORT_TEAM: Permissions = Permissions::SUPPORT_TEAM;

const MAX_TITLE: usize = 128;
const MAX_SUBJECT: usize = 256;
const MAX_CONTENT: usize = 20_000;
/// Files per message.
const MAX_ATTACHMENTS: usize = 10;
const MAX_ASSOCIATED_USERS: usize = 10;
const MAX_FAQ_LIMIT: i64 = 100;

/// Query string of the ticket lists. Every field is optional, and empty values are
/// ignored, so a plain HTML form can submit all its fields.
#[derive(Deserialize)]
struct TicketQuery {
    status: Option<String>,
    /// `active` or `done`, for the "on-going" and "resolved/closed" lists.
    state: Option<String>,
    category: Option<String>,
    /// `day`, `week` or `month`: updated within that window.
    activity: Option<String>,
    /// `latest` (default), `oldest` or `created`.
    sort: Option<String>,
}

/// The current user's tickets, filtered by the query. See [`parse_filter`].
#[get("/support/tickets")]
async fn list_my_tickets(
    state: web::Data<AppState>,
    user: AuthUser,
    query: web::Query<TicketQuery>,
) -> Result<HttpResponse, ApiError> {
    let mut filter = parse_filter(query.into_inner())?;
    filter.user_id = Some(user.0.id);
    let tickets = SupportTicket::list(&state.db, &filter, false).await?;
    Ok(HttpResponse::Ok().json(tickets))
}

/// Every ticket, filtered by the query. Requires Support Team.
#[get("/support/admin/tickets")]
async fn list_all_tickets(
    state: web::Data<AppState>,
    user: AuthUser,
    query: web::Query<TicketQuery>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SUPPORT_TEAM).await?;
    let filter = parse_filter(query.into_inner())?;
    let tickets = SupportTicket::list(&state.db, &filter, true).await?;
    Ok(HttpResponse::Ok().json(tickets))
}

#[derive(Deserialize)]
struct TicketBody {
    title: String,
    subject: String,
    category: TicketCategory,
    #[serde(default)]
    associated_user_ids: Vec<i64>,
    /// The initial message, Markdown.
    content: String,
    /// IDs of files of the user's to attach to the initial message.
    #[serde(default)]
    attachments: Vec<String>,
}

/// Opens a ticket with its initial message. Any logged-in user; 400 once they have
/// [`MAX_ACTIVE_TICKETS`] tickets in progress.
#[post("/support/tickets")]
async fn create_ticket(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<TicketBody>,
) -> Result<HttpResponse, ApiError> {
    let mut details = normalize_ticket(body.into_inner(), user.0.id)?;
    details.attachments = files::attachable(&state, user.0.id, &details.attachments, MAX_ATTACHMENTS).await?;
    let missing = User::missing_ids(&state.db, &details.associated_user_ids).await?;
    if let Some(id) = missing.first() {
        return Err(bad(format!("associated user {id} does not exist")));
    }
    if SupportTicket::count_active(&state.db, user.0.id).await? >= MAX_ACTIVE_TICKETS {
        return Err(bad(format!(
            "you already have {MAX_ACTIVE_TICKETS} tickets in progress; wait for one to be resolved or closed"
        )));
    }
    let ticket = SupportTicket::create(&state.db, user.0.id, &details).await?;
    log::info!("user {} opened support ticket {} ({:?})", user.0.id, ticket.id, ticket.title);
    Ok(HttpResponse::Created().json(ticket))
}

#[derive(Serialize)]
struct TicketPage {
    ticket: SupportTicket,
    author: UserRef,
    associated_users: Vec<UserRef>,
    responders: Vec<UserRef>,
    /// Who resolved or closed the ticket, if someone did (not automatic closing).
    closed_by_user: Option<UserRef>,
    /// Oldest first. Internal messages only for the Support Team.
    messages: Vec<MessageWithAuthor>,
    /// Whether the requester is on the Support Team (so sees internal messages and tools).
    support: bool,
}

/// A ticket with its messages and everyone involved. For its owner and the Support Team.
#[get("/support/tickets/{id}")]
async fn get_ticket(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let (ticket, support) = find_ticket(&state, &user, id.into_inner()).await?;
    let owner = [ticket.user_id];
    let (author, associated_users, responders, closed_by_user, messages) = tokio::try_join!(
        UserRef::by_ids(&state.db, &owner),
        UserRef::by_ids(&state.db, &ticket.associated_user_ids),
        UserRef::by_ids(&state.db, &ticket.responders),
        UserRef::by_ids(&state.db, ticket.closed_by.as_slice()),
        ticket.messages(&state.db, support),
    )?;
    let author = author.into_iter().next().ok_or(ApiError::NotFound("user"))?;
    Ok(HttpResponse::Ok().json(TicketPage {
        ticket,
        author,
        associated_users,
        responders,
        closed_by_user: closed_by_user.into_iter().next(),
        messages,
        support,
    }))
}

#[derive(Deserialize)]
struct MessageBody {
    content: String,
    /// IDs of files of the user's to attach.
    #[serde(default)]
    attachments: Vec<String>,
    /// Support Team only: a note other team members can see but the user can't.
    #[serde(default)]
    is_internal: bool,
}

/// Adds a message to a ticket. The owner can reply while the ticket is in progress (423
/// once resolved or closed), moving it to `AWAITING_ADMIN`. The Support Team can always
/// reply, moving it to `AWAITING_USER`, or leave an internal note.
#[post("/support/tickets/{id}/messages")]
async fn create_message(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<MessageBody>,
) -> Result<HttpResponse, ApiError> {
    let content = markdown("content", &body.content, MAX_CONTENT)?;
    let (ticket, support) = find_ticket(&state, &user, id.into_inner()).await?;
    let attachments = files::attachable(&state, user.0.id, &body.attachments, MAX_ATTACHMENTS).await?;
    let message = if support {
        ticket
            .reply_as_support(&state.db, user.0.id, &content, body.is_internal, &attachments)
            .await?
    } else {
        if body.is_internal {
            return Err(ApiError::Forbidden(SUPPORT_TEAM.name().to_owned()));
        }
        if ticket.status.is_done() {
            return Err(ApiError::Locked("ticket"));
        }
        ticket.reply_as_user(&state.db, &content, &attachments).await?
    };
    Ok(HttpResponse::Created().json(message))
}

#[derive(Deserialize)]
struct StatusBody {
    status: SupportStatus,
}

/// Sets a ticket's status: `RESOLVED` or `CLOSED` to finish it, anything else to re-open
/// it. Requires Support Team.
#[patch("/support/tickets/{id}/status")]
async fn set_status(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<StatusBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SUPPORT_TEAM).await?;
    let ticket = SupportTicket::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("ticket"))?;
    if ticket.status == body.status {
        return Err(bad(format!("ticket is already {}", format!("{:?}", body.status).to_uppercase())));
    }
    let updated = ticket.set_status(&state.db, body.status, user.0.id).await?;
    log::info!(
        "user {} set support ticket {} to {:?} (was {:?})",
        user.0.id, ticket.id, updated.status, ticket.status
    );
    Ok(HttpResponse::Ok().json(updated))
}

#[derive(Deserialize)]
struct EditBody {
    content: String,
}

/// Edits a message's content, once. Authors only; a ticket owner can't edit once the
/// ticket is resolved or closed.
#[patch("/support/messages/{id}")]
async fn edit_message(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<EditBody>,
) -> Result<HttpResponse, ApiError> {
    let content = markdown("content", &body.content, MAX_CONTENT)?;
    let message = SupportMessage::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("message"))?;
    if message.author_id != user.0.id {
        return Err(ApiError::Forbidden("message author".to_owned()));
    }
    if message.previous_content.is_some() {
        return Err(bad("messages can only be edited once".to_owned()));
    }
    let (ticket, support) = find_ticket(&state, &user, message.ticket_id).await?;
    if !support && ticket.status.is_done() {
        return Err(ApiError::Locked("ticket"));
    }
    let edited = message.edit(&state.db, &content).await?;
    Ok(HttpResponse::Ok().json(edited))
}

#[derive(Deserialize)]
struct FaqQuery {
    category: Option<String>,
    limit: Option<String>,
}

/// FAQs, most recently updated first; `category` narrows them and `limit` (1 to 100) caps
/// them. Public.
#[get("/support/faqs")]
async fn list_faqs(
    state: web::Data<AppState>,
    query: web::Query<FaqQuery>,
) -> Result<HttpResponse, ApiError> {
    let query = query.into_inner();
    let category = present(query.category)
        .map(|v| parse_enum::<FaqCategory>("category", &v))
        .transpose()?;
    let limit = present(query.limit)
        .map(|v| v.parse::<i64>().map_err(|_| bad("limit must be a number".to_owned())))
        .transpose()?;
    if limit.is_some_and(|l| !(1..=MAX_FAQ_LIMIT).contains(&l)) {
        return Err(bad(format!("limit must be between 1 and {MAX_FAQ_LIMIT}")));
    }
    let faqs = Faq::list(&state.db, category, limit).await?;
    Ok(HttpResponse::Ok().json(faqs))
}

/// One FAQ in full, with its creator and editors. Public.
#[get("/support/faqs/{id}")]
async fn get_faq(state: web::Data<AppState>, id: web::Path<i64>) -> Result<HttpResponse, ApiError> {
    let faq = find_faq(&state, id.into_inner()).await?;
    Ok(HttpResponse::Ok().json(faq))
}

#[derive(Deserialize)]
struct FaqBody {
    title: String,
    subject: String,
    category: FaqCategory,
    /// Markdown.
    description: String,
}

/// Creates an FAQ. Requires Support Team.
#[post("/support/faqs")]
async fn create_faq(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<FaqBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SUPPORT_TEAM).await?;
    let details = normalize_faq(body.into_inner())?;
    let faq = Faq::create(&state.db, user.0.id, &details).await?;
    log::info!("user {} created FAQ {} ({:?})", user.0.id, faq.id, faq.title);
    Ok(HttpResponse::Created().json(find_faq(&state, faq.id).await?))
}

/// Replaces an FAQ's details, recording the editor. Requires Support Team.
#[put("/support/faqs/{id}")]
async fn update_faq(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<FaqBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SUPPORT_TEAM).await?;
    let details = normalize_faq(body.into_inner())?;
    let faq = Faq::update(&state.db, id.into_inner(), user.0.id, &details)
        .await?
        .ok_or(ApiError::NotFound("FAQ"))?;
    log::info!("user {} edited FAQ {} ({:?})", user.0.id, faq.id, faq.title);
    Ok(HttpResponse::Ok().json(find_faq(&state, faq.id).await?))
}

/// Deletes an FAQ. Requires Support Team.
#[delete("/support/faqs/{id}")]
async fn delete_faq(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, SUPPORT_TEAM).await?;
    let id = id.into_inner();
    if !Faq::delete(&state.db, id).await? {
        return Err(ApiError::NotFound("FAQ"));
    }
    log::info!("user {} deleted FAQ {id}", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

/// The ticket, and whether `user` is on the Support Team. 404 (not 403) for other
/// people's tickets, so ids can't be probed.
async fn find_ticket(
    state: &AppState,
    user: &AuthUser,
    id: i64,
) -> Result<(SupportTicket, bool), ApiError> {
    let ticket = SupportTicket::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("ticket"))?;
    let support = user.require(&state.db, SUPPORT_TEAM).await.is_ok();
    if !support && ticket.user_id != user.0.id {
        return Err(ApiError::NotFound("ticket"));
    }
    Ok((ticket, support))
}

async fn find_faq(state: &AppState, id: i64) -> Result<FaqWithPeople, ApiError> {
    Faq::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("FAQ"))
}

/// `None` for a missing or blank value, else the trimmed value.
fn present(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

/// Parses an enum from its SCREAMING_SNAKE_CASE name (case-insensitively), as serde would.
fn parse_enum<T: serde::de::DeserializeOwned>(field: &str, value: &str) -> Result<T, ApiError> {
    serde_json::from_value(serde_json::Value::String(value.to_ascii_uppercase()))
        .map_err(|_| bad(format!("{field} is not a known {field}: {value:?}")))
}

fn parse_filter(query: TicketQuery) -> Result<TicketFilter, ApiError> {
    let active = match present(query.state).as_deref() {
        None => None,
        Some("active") => Some(true),
        Some("done") => Some(false),
        Some(_) => return Err(bad("state must be active or done".to_owned())),
    };
    let updated_within_days = match present(query.activity).as_deref() {
        None => None,
        Some("day") => Some(1),
        Some("week") => Some(7),
        Some("month") => Some(30),
        Some(_) => return Err(bad("activity must be one of day, week, month".to_owned())),
    };
    let sort = match present(query.sort).as_deref() {
        None | Some("latest") => TicketSort::Latest,
        Some("oldest") => TicketSort::Oldest,
        Some("created") => TicketSort::Created,
        Some(_) => return Err(bad("sort must be one of latest, oldest, created".to_owned())),
    };
    Ok(TicketFilter {
        user_id: None,
        status: present(query.status)
            .map(|v| parse_enum::<SupportStatus>("status", &v))
            .transpose()?,
        active,
        category: present(query.category)
            .map(|v| parse_enum::<TicketCategory>("category", &v))
            .transpose()?,
        updated_within_days,
        sort,
    })
}

fn normalize_ticket(body: TicketBody, user_id: i64) -> Result<TicketDetails, ApiError> {
    let mut associated_user_ids = body.associated_user_ids;
    associated_user_ids.sort_unstable();
    associated_user_ids.dedup();
    associated_user_ids.retain(|&id| id != user_id);
    if associated_user_ids.len() > MAX_ASSOCIATED_USERS {
        return Err(bad(format!("at most {MAX_ASSOCIATED_USERS} users can be associated with a ticket")));
    }
    Ok(TicketDetails {
        title: required_text("title", &body.title, MAX_TITLE)?,
        subject: required_text("subject", &body.subject, MAX_SUBJECT)?,
        category: body.category,
        associated_user_ids,
        content: markdown("content", &body.content, MAX_CONTENT)?,
        // Checked against the database by the route.
        attachments: body.attachments,
    })
}

fn normalize_faq(body: FaqBody) -> Result<FaqDetails, ApiError> {
    Ok(FaqDetails {
        title: required_text("title", &body.title, MAX_TITLE)?,
        subject: required_text("subject", &body.subject, MAX_SUBJECT)?,
        category: body.category,
        description: markdown("description", &body.description, MAX_CONTENT)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tickets() {
        let body = || TicketBody {
            title: " Lost my item ".into(),
            subject: "Bought a hat, never got it".into(),
            category: TicketCategory::UserIssue,
            associated_user_ids: vec![3, 1, 3, 7],
            content: "Details here".into(),
            attachments: Vec::new(),
        };
        let details = normalize_ticket(body(), 7).unwrap_or_else(|_| panic!("valid body rejected"));
        assert_eq!(details.title, "Lost my item");
        assert_eq!(details.associated_user_ids, vec![1, 3]);

        assert!(normalize_ticket(TicketBody { title: " ".into(), ..body() }, 7).is_err());
        assert!(normalize_ticket(TicketBody { subject: "x".repeat(257), ..body() }, 7).is_err());
        assert!(normalize_ticket(TicketBody { content: "<script>".into(), ..body() }, 7).is_ok());
        assert!(normalize_ticket(TicketBody { associated_user_ids: (10..=20).collect(), ..body() }, 7).is_err());
    }

    #[test]
    fn filters() {
        let query = |status: &str, state: &str, sort: &str| TicketQuery {
            status: Some(status.into()),
            state: Some(state.into()),
            category: None,
            activity: Some("week".into()),
            sort: Some(sort.into()),
        };
        let filter = parse_filter(query("awaiting_admin", "active", "oldest")).unwrap();
        assert_eq!(filter.status, Some(SupportStatus::AwaitingAdmin));
        assert_eq!(filter.active, Some(true));
        assert_eq!(filter.updated_within_days, Some(7));
        assert_eq!(filter.sort, TicketSort::Oldest);

        let filter = parse_filter(query("", "", "")).unwrap();
        assert_eq!(filter.status, None);
        assert_eq!(filter.sort, TicketSort::Latest);

        assert!(parse_filter(query("PENDING", "", "")).is_err());
        assert!(parse_filter(query("", "maybe", "")).is_err());
        assert!(parse_filter(query("", "", "random")).is_err());
    }

    #[test]
    fn enums() {
        assert_eq!(parse_enum::<FaqCategory>("category", "store").unwrap(), FaqCategory::Store);
        assert_eq!(parse_enum::<TicketCategory>("category", "BUG_REPORT").unwrap(), TicketCategory::BugReport);
        assert!(parse_enum::<FaqCategory>("category", "nope").is_err());
    }
}
