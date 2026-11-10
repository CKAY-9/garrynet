//! The forum; see docs/FORUM.md. Reading is public. Sections and categories are managed
//! by admins; threads and replies need User Posts. Authors manage their own threads and
//! posts; Thread Management manages anyone's.

use actix_web::{HttpResponse, delete, get, patch, post, web};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, User, Usergroup,
        forum::{
            CategoryDetails, ForumCategory, ForumPost, ForumSection, ForumStats, ForumThread,
            PostWithAuthor, SectionDetails, SectionWithCategories, ThreadChanges, ThreadSummary,
        },
    },
    routes::{
        markdown::markdown,
        validate::{bad, hex_color, optional_url, required_text, text},
    },
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(home)
        .service(create_section)
        .service(delete_section)
        .service(create_category)
        .service(delete_category)
        .service(get_category)
        .service(create_thread)
        .service(get_thread)
        .service(update_thread)
        .service(delete_thread)
        .service(create_post)
        .service(edit_post)
        .service(delete_post);
}

/// Like the other admin pages, managing the forum needs Admin Access as well as the
/// specific permission.
const MANAGE_SECTIONS: Permissions =
    Permissions::ADMIN_ACCESS.union(Permissions::SECTION_MANAGEMENT);
const MANAGE_CATEGORIES: Permissions =
    Permissions::ADMIN_ACCESS.union(Permissions::CATEGORY_MANAGEMENT);
/// Site Access too: without it a user is banned.
const POST: Permissions = Permissions::SITE_ACCESS.union(Permissions::USER_POSTS);
/// Moderating threads and posts happens on the forum itself, so it doesn't need Admin
/// Access; Site Access still does, so a banned moderator can't moderate.
const MODERATE: Permissions = Permissions::SITE_ACCESS.union(Permissions::THREAD_MANAGEMENT);

const MAX_NAME: usize = 64;
const MAX_DESCRIPTION: usize = 256;
const MAX_TITLE: usize = 128;
const MAX_CONTENT: usize = 20_000;
const DEFAULT_ACCENT: &str = "#3b82f6";

#[derive(Serialize)]
struct Home {
    sections: Vec<SectionWithCategories>,
    stats: HomeStats,
}

#[derive(Serialize)]
struct HomeStats {
    threads: i64,
    posts: i64,
    members: i64,
    online: i64,
}

/// Every section with its categories, plus forum statistics. Public.
#[get("/forum")]
async fn home(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let (sections, stats, members, online) = tokio::try_join!(
        ForumSection::list_with_categories(&state.db),
        ForumStats::get(&state.db),
        User::count(&state.db),
        User::count_online(&state.db),
    )?;
    Ok(HttpResponse::Ok().json(Home {
        sections,
        stats: HomeStats { threads: stats.threads, posts: stats.posts, members, online },
    }))
}

#[derive(Deserialize)]
struct SectionBody {
    name: String,
    accent: Option<String>,
    icon_url: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    position: i32,
}

/// Creates a section. Requires Admin Access and Section Management.
#[post("/forum/sections")]
async fn create_section(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<SectionBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE_SECTIONS).await?;
    let details = normalize_section(body.into_inner())?;
    let section = ForumSection::create(&state.db, &details).await?;
    log::info!("user {} created forum section {} ({:?})", user.0.id, section.id, section.name);
    Ok(HttpResponse::Created().json(section))
}

/// Deletes a section with all its categories, threads and posts. Requires Admin Access
/// and Section Management.
#[delete("/forum/sections/{id}")]
async fn delete_section(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE_SECTIONS).await?;
    let id = id.into_inner();
    if !ForumSection::delete(&state.db, id).await? {
        return Err(ApiError::NotFound("section"));
    }
    log::info!("user {} deleted forum section {id}", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
struct CategoryBody {
    name: String,
    section_id: i64,
    #[serde(default)]
    description: String,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    position: i32,
}

/// Creates a category in an existing section. Requires Admin Access and Category
/// Management.
#[post("/forum/categories")]
async fn create_category(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<CategoryBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE_CATEGORIES).await?;
    let details = normalize_category(body.into_inner())?;
    if ForumSection::find_by_id(&state.db, details.section_id).await?.is_none() {
        return Err(bad(format!("section {} does not exist", details.section_id)));
    }
    let category = ForumCategory::create(&state.db, &details).await?;
    log::info!("user {} created forum category {} ({:?})", user.0.id, category.id, category.name);
    Ok(HttpResponse::Created().json(category))
}

/// Deletes a category with its threads and posts. Requires Admin Access and Category
/// Management.
#[delete("/forum/categories/{id}")]
async fn delete_category(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE_CATEGORIES).await?;
    let id = id.into_inner();
    if !ForumCategory::delete(&state.db, id).await? {
        return Err(ApiError::NotFound("category"));
    }
    log::info!("user {} deleted forum category {id}", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize)]
struct CategoryPage {
    category: ForumCategory,
    section: ForumSection,
    threads: Vec<ThreadSummary>,
}

/// A category with its section and threads (stickies first, then most recently active).
/// Public.
#[get("/forum/categories/{id}")]
async fn get_category(
    state: web::Data<AppState>,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let (category, section) = category_and_section(&state, id.into_inner()).await?;
    let threads = ForumCategory::threads(&state.db, category.id).await?;
    Ok(HttpResponse::Ok().json(CategoryPage { category, section, threads }))
}

#[derive(Deserialize)]
struct ThreadBody {
    title: String,
    content: String,
}

/// Starts a thread with its first post. Requires Site Access and User Posts; 423 if the
/// category is locked.
#[post("/forum/categories/{id}/threads")]
async fn create_thread(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<ThreadBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, POST).await?;
    let title = required_text("title", &body.title, MAX_TITLE)?;
    let content = markdown("content", &body.content, MAX_CONTENT)?;
    let category = ForumCategory::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("category"))?;
    if category.locked {
        return Err(ApiError::Locked("category"));
    }
    let thread = ForumThread::create(&state.db, user.0.id, category.id, &title, &content).await?;
    log::info!("user {} started forum thread {} in category {}", user.0.id, thread.id, category.id);
    Ok(HttpResponse::Created().json(thread))
}

#[derive(Serialize)]
struct ThreadPage {
    category: ForumCategory,
    section: ForumSection,
    thread: ForumThread,
    posts: Vec<PostWithAuthor>,
}

/// A thread with its category, section and posts (the first post, then replies oldest
/// first). Public.
#[get("/forum/threads/{id}")]
async fn get_thread(
    state: web::Data<AppState>,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let thread = ForumThread::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("thread"))?;
    let (category, section) = category_and_section(&state, thread.category_id).await?;
    let posts = thread.posts(&state.db).await?;
    Ok(HttpResponse::Ok().json(ThreadPage { category, section, thread, posts }))
}

#[derive(Deserialize)]
struct PostBody {
    content: String,
}

/// Replies to a thread. Requires Site Access and User Posts; 423 if the thread or its
/// category is locked.
#[post("/forum/threads/{id}/posts")]
async fn create_post(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<PostBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, POST).await?;
    let content = markdown("content", &body.content, MAX_CONTENT)?;
    let thread = ForumThread::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("thread"))?;
    if thread.locked {
        return Err(ApiError::Locked("thread"));
    }
    let category = ForumCategory::find_by_id(&state.db, thread.category_id)
        .await?
        .ok_or(ApiError::NotFound("category"))?;
    if category.locked {
        return Err(ApiError::Locked("category"));
    }
    let post = thread.reply(&state.db, user.0.id, &content).await?;
    Ok(HttpResponse::Created().json(post))
}

#[derive(Deserialize)]
struct ThreadUpdateBody {
    title: Option<String>,
    locked: Option<bool>,
    sticky: Option<bool>,
    allow_template: Option<bool>,
}

/// Changes a thread's title, lock, sticky or template flag; omitted fields stay as they
/// are. Authors can change their own thread's title and template flag (see [`manager`]);
/// locking and stickying need Thread Management.
#[patch("/forum/threads/{id}")]
async fn update_thread(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<ThreadUpdateBody>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let changes = ThreadChanges {
        title: body.title.map(|t| required_text("title", &t, MAX_TITLE)).transpose()?,
        locked: body.locked,
        sticky: body.sticky,
        allow_template: body.allow_template,
    };
    if changes.title.is_none()
        && changes.locked.is_none()
        && changes.sticky.is_none()
        && changes.allow_template.is_none()
    {
        return Err(bad("nothing to change".to_owned()));
    }
    let thread = find_thread(&state, id.into_inner()).await?;
    let role = manager(&state, &user, thread.author_id, &thread).await?;
    if role == Manager::Author && (changes.locked.is_some() || changes.sticky.is_some()) {
        return Err(ApiError::Forbidden(Permissions::THREAD_MANAGEMENT.name().to_owned()));
    }
    let updated = thread.update(&state.db, &changes).await?;
    log::info!("user {} updated forum thread {} ({changes:?})", user.0.id, thread.id);
    Ok(HttpResponse::Ok().json(updated))
}

/// Deletes a thread with all its posts. Authors can delete their own (see [`manager`]).
#[delete("/forum/threads/{id}")]
async fn delete_thread(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let thread = find_thread(&state, id.into_inner()).await?;
    manager(&state, &user, thread.author_id, &thread).await?;
    thread.delete(&state.db).await?;
    log::info!("user {} deleted forum thread {} ({:?})", user.0.id, thread.id, thread.title);
    Ok(HttpResponse::NoContent().finish())
}

/// Replaces a post's content and marks it edited. Authors can edit their own (see
/// [`manager`]).
#[patch("/forum/posts/{id}")]
async fn edit_post(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<PostBody>,
) -> Result<HttpResponse, ApiError> {
    let content = markdown("content", &body.content, MAX_CONTENT)?;
    let (post, thread) = find_post(&state, id.into_inner()).await?;
    manager(&state, &user, post.author_id, &thread).await?;
    let edited = post.edit(&state.db, &content).await?;
    if user.0.id != post.author_id {
        log::info!("user {} edited forum post {} by user {}", user.0.id, post.id, post.author_id);
    }
    Ok(HttpResponse::Ok().json(edited))
}

/// Deletes a post. Deleting a thread's first post deletes the whole thread. Authors can
/// delete their own (see [`manager`]).
#[delete("/forum/posts/{id}")]
async fn delete_post(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let (post, thread) = find_post(&state, id.into_inner()).await?;
    manager(&state, &user, post.author_id, &thread).await?;
    if post.id == thread.thread_post_id {
        thread.delete(&state.db).await?;
        log::info!("user {} deleted forum thread {} with its first post", user.0.id, thread.id);
    } else {
        post.delete_reply(&state.db).await?;
        log::info!(
            "user {} deleted forum post {} by user {} in thread {}",
            user.0.id, post.id, post.author_id, thread.id
        );
    }
    Ok(HttpResponse::NoContent().finish())
}

/// How a user may manage a thread or post.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Manager {
    /// Has Thread Management: can manage anything, locked or not.
    Moderator,
    /// Wrote it, and may still post.
    Author,
}

/// Whether `user` may manage a thread or post written by `author_id` in `thread`.
/// Moderators (Site Access + Thread Management) can manage anything. Authors can manage
/// their own as long as they can still post (Site Access + User Posts, so not banned)
/// and neither the thread nor its category is locked.
async fn manager(
    state: &AppState,
    user: &AuthUser,
    author_id: i64,
    thread: &ForumThread,
) -> Result<Manager, ApiError> {
    let permissions = Usergroup::permissions_for_user(&state.db, user.0.id).await?;
    if permissions.contains(MODERATE) {
        return Ok(Manager::Moderator);
    }
    if author_id != user.0.id {
        return Err(ApiError::Forbidden(Permissions::THREAD_MANAGEMENT.name().to_owned()));
    }
    for flag in [Permissions::SITE_ACCESS, Permissions::USER_POSTS] {
        if !permissions.contains(flag) {
            return Err(ApiError::Forbidden(flag.name().to_owned()));
        }
    }
    if thread.locked {
        return Err(ApiError::Locked("thread"));
    }
    let category = ForumCategory::find_by_id(&state.db, thread.category_id)
        .await?
        .ok_or(ApiError::NotFound("category"))?;
    if category.locked {
        return Err(ApiError::Locked("category"));
    }
    Ok(Manager::Author)
}

async fn find_thread(state: &AppState, id: i64) -> Result<ForumThread, ApiError> {
    ForumThread::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("thread"))
}

/// A post and the thread it belongs to.
async fn find_post(state: &AppState, id: i64) -> Result<(ForumPost, ForumThread), ApiError> {
    let post = ForumPost::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("post"))?;
    let thread = find_thread(state, post.thread_id).await?;
    Ok((post, thread))
}

async fn category_and_section(
    state: &AppState,
    category_id: i64,
) -> Result<(ForumCategory, ForumSection), ApiError> {
    let category = ForumCategory::find_by_id(&state.db, category_id)
        .await?
        .ok_or(ApiError::NotFound("category"))?;
    let section = ForumSection::find_by_id(&state.db, category.section_id)
        .await?
        .ok_or(ApiError::NotFound("section"))?;
    Ok((category, section))
}

/// Positions are listed highest first; -1 means always last.
fn position(value: i32) -> Result<i32, ApiError> {
    if value < -1 {
        return Err(bad("position must be -1 or greater".to_owned()));
    }
    Ok(value)
}

fn normalize_section(body: SectionBody) -> Result<SectionDetails, ApiError> {
    Ok(SectionDetails {
        name: required_text("name", &body.name, MAX_NAME)?,
        accent: match body.accent.as_deref().map(str::trim) {
            None | Some("") => DEFAULT_ACCENT.to_owned(),
            Some(accent) => hex_color("accent", accent)?,
        },
        icon_url: optional_url("icon_url", body.icon_url.as_deref())?,
        description: text("description", &body.description, MAX_DESCRIPTION)?,
        position: position(body.position)?,
    })
}

fn normalize_category(body: CategoryBody) -> Result<CategoryDetails, ApiError> {
    Ok(CategoryDetails {
        name: required_text("name", &body.name, MAX_NAME)?,
        section_id: body.section_id,
        description: text("description", &body.description, MAX_DESCRIPTION)?,
        locked: body.locked,
        position: position(body.position)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections() {
        let body = || SectionBody {
            name: " General ".into(),
            accent: None,
            icon_url: Some("".into()),
            description: String::new(),
            position: 0,
        };
        let details = normalize_section(body()).unwrap_or_else(|_| panic!("valid body rejected"));
        assert_eq!(details.name, "General");
        assert_eq!(details.accent, DEFAULT_ACCENT);
        assert_eq!(details.icon_url, None);

        assert!(normalize_section(SectionBody { position: -2, ..body() }).is_err());
        assert!(normalize_section(SectionBody { accent: Some("red".into()), ..body() }).is_err());
        assert!(normalize_section(SectionBody { description: "x".repeat(257), ..body() }).is_err());
        assert!(normalize_section(SectionBody { name: " ".into(), ..body() }).is_err());
    }
}
