//! The forum; see docs/FORUM.md. Reading is public. Sections and categories are managed
//! by admins; threads and replies need User Posts. Authors manage their own threads and
//! posts; Thread Management manages anyone's, except those of users holding permissions the
//! moderator doesn't.

use actix_web::{HttpResponse, delete, get, patch, post, put, web};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        File, Permissions, User, Usergroup,
        forum::{
            CategoryDetails, ForumCategory, ForumPost, ForumSection, ForumStats, ForumThread,
            PostWithAuthor, SectionDetails, SectionWithCategories, ThreadChanges, ThreadSummary, Vote,
        },
    },
    notify,
    routes::{
        files,
        markdown::markdown,
        validate::{bad, hex_color, optional_url, present, rate_limit, required_text, text},
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
        .service(vote_post)
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
/// Files per post.
const MAX_ATTACHMENTS: usize = 5;
const DEFAULT_ACCENT: &str = "#3b82f6";

/// Threads one user may start in any [`THREAD_WINDOW_SECS`]...
const THREAD_LIMIT: i64 = 3;
const THREAD_WINDOW_SECS: f64 = 10.0 * 60.0;
/// ...and posts (threads' first posts and replies) they may write in any
/// [`POST_WINDOW_SECS`]. Moderators aren't limited.
const POST_LIMIT: i64 = 6;
const POST_WINDOW_SECS: f64 = 60.0;

/// Threads per page of a category, and posts per page of a thread.
const THREADS_PER_PAGE: i64 = 25;
const POSTS_PER_PAGE: i64 = 20;

/// Which page of a list to show: the requested `page` (from 1), moved back to the last page
/// if the list is shorter. Returns the page and how many pages `total` items fill (at least
/// 1, so an empty list has one empty page). Requests past the end can't make the database
/// skip more rows than the list has.
fn clamp_page(page: i64, total: i64, per_page: i64) -> (i64, i64) {
    let pages = ((total + per_page - 1) / per_page).max(1);
    (page.clamp(1, pages), pages)
}

/// The `page` query parameter: from 1, default 1.
fn parse_page(value: Option<String>) -> Result<i64, ApiError> {
    let Some(value) = present(value) else { return Ok(1) };
    match value.parse::<i64>() {
        Ok(page) if page >= 1 => Ok(page),
        _ => Err(bad("page must be a whole number, 1 or greater".to_owned())),
    }
}

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

/// Deletes a section with all its categories, threads and posts, and their attachments.
/// Requires Admin Access and Section Management.
#[delete("/forum/sections/{id}")]
async fn delete_section(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE_SECTIONS).await?;
    let id = id.into_inner();
    let attached = File::of_forum_section(&state.db, id).await?;
    if !ForumSection::delete(&state.db, id).await? {
        return Err(ApiError::NotFound("section"));
    }
    files::delete_all(&state, attached).await;
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

/// Deletes a category with its threads and posts, and their attachments. Requires Admin
/// Access and Category Management.
#[delete("/forum/categories/{id}")]
async fn delete_category(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE_CATEGORIES).await?;
    let id = id.into_inner();
    let attached = File::of_forum_category(&state.db, id).await?;
    if !ForumCategory::delete(&state.db, id).await? {
        return Err(ApiError::NotFound("category"));
    }
    files::delete_all(&state, attached).await;
    log::info!("user {} deleted forum category {id}", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
struct PageQuery {
    page: Option<String>,
}

#[derive(Serialize)]
struct CategoryPage {
    category: ForumCategory,
    section: ForumSection,
    threads: Vec<ThreadSummary>,
    /// The page shown, from 1, and how many there are.
    page: i64,
    pages: i64,
}

/// A category with its section and one page of its threads ([`THREADS_PER_PAGE`]; stickies
/// first, then most recently active). `?page=` picks the page, default 1; past the end shows
/// the last. Public.
#[get("/forum/categories/{id}")]
async fn get_category(
    state: web::Data<AppState>,
    id: web::Path<i64>,
    query: web::Query<PageQuery>,
) -> Result<HttpResponse, ApiError> {
    let requested = parse_page(query.into_inner().page)?;
    let (category, section) = category_and_section(&state, id.into_inner()).await?;
    let total = ForumCategory::thread_count(&state.db, category.id).await?;
    let (page, pages) = clamp_page(requested, total, THREADS_PER_PAGE);
    let threads =
        ForumCategory::threads(&state.db, category.id, THREADS_PER_PAGE, (page - 1) * THREADS_PER_PAGE).await?;
    Ok(HttpResponse::Ok().json(CategoryPage { category, section, threads, page, pages }))
}

#[derive(Deserialize)]
struct ThreadBody {
    title: String,
    content: String,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    allow_template: bool,
    /// IDs of files of the user's to attach to the first post.
    #[serde(default)]
    attachments: Vec<String>,
}

/// Starts a thread with its first post. Requires Site Access and User Posts; starting it
/// locked also needs Thread Management. 423 if the category is locked; 429 past the rate
/// limit (see [`check_rate`]).
#[post("/forum/categories/{id}/threads")]
async fn create_thread(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<ThreadBody>,
) -> Result<HttpResponse, ApiError> {
    let permissions = user.require(&state.db, POST).await?;
    if body.locked && !permissions.contains(MODERATE) {
        return Err(ApiError::Forbidden(Permissions::THREAD_MANAGEMENT.name().to_owned()));
    }
    let title = required_text("title", &body.title, MAX_TITLE)?;
    let content = markdown("content", &body.content, MAX_CONTENT)?;
    let category = ForumCategory::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("category"))?;
    if category.locked {
        return Err(ApiError::Locked("category"));
    }
    if !permissions.contains(MODERATE) {
        check_rate(&state, user.0.id, true).await?;
    }
    let attachments = files::attachable(&state, user.0.id, &body.attachments, MAX_ATTACHMENTS).await?;
    let thread = ForumThread::create(
        &state.db,
        user.0.id,
        category.id,
        &title,
        &content,
        body.locked,
        body.allow_template,
        &attachments,
    )
    .await?;
    log::info!("user {} started forum thread {} in category {}", user.0.id, thread.id, category.id);
    Ok(HttpResponse::Created().json(thread))
}

#[derive(Deserialize)]
struct ThreadQuery {
    page: Option<String>,
    post: Option<String>,
}

#[derive(Serialize)]
struct ThreadPage {
    category: ForumCategory,
    section: ForumSection,
    thread: ForumThread,
    posts: Vec<PostWithAuthor>,
    /// The page shown, from 1, and how many there are.
    page: i64,
    pages: i64,
}

/// A thread with its category, section and one page of its posts ([`POSTS_PER_PAGE`]; the
/// first post, then replies oldest first). `?page=` picks the page, default 1; past the end
/// shows the last. `?post={id}` instead shows the page holding that post, or the first page
/// if the thread has no such post (it may have been deleted). Public.
#[get("/forum/threads/{id}")]
async fn get_thread(
    state: web::Data<AppState>,
    id: web::Path<i64>,
    query: web::Query<ThreadQuery>,
) -> Result<HttpResponse, ApiError> {
    let query = query.into_inner();
    let page = parse_page(query.page)?;
    let post = present(query.post)
        .map(|v| v.parse::<i64>().map_err(|_| bad("post must be a post id".to_owned())))
        .transpose()?;

    let thread = ForumThread::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("thread"))?;
    let (category, section) = category_and_section(&state, thread.category_id).await?;
    let requested = match post {
        Some(post) => thread.posts_before(&state.db, post).await?.map_or(1, |before| before / POSTS_PER_PAGE + 1),
        None => page,
    };
    let total = thread.post_count(&state.db).await?;
    let (page, pages) = clamp_page(requested, total, POSTS_PER_PAGE);
    let posts = thread.posts(&state.db, POSTS_PER_PAGE, (page - 1) * POSTS_PER_PAGE).await?;
    Ok(HttpResponse::Ok().json(ThreadPage { category, section, thread, posts, page, pages }))
}

#[derive(Deserialize)]
struct PostBody {
    content: String,
    /// IDs of files of the user's to attach.
    #[serde(default)]
    attachments: Vec<String>,
}

/// Editing replaces the content only; attachments stay as they are.
#[derive(Deserialize)]
struct PostEditBody {
    content: String,
}

/// Replies to a thread. Requires Site Access and User Posts; 423 if the thread or its
/// category is locked; 429 past the rate limit (see [`check_rate`]).
#[post("/forum/threads/{id}/posts")]
async fn create_post(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<PostBody>,
) -> Result<HttpResponse, ApiError> {
    let permissions = user.require(&state.db, POST).await?;
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
    if !permissions.contains(MODERATE) {
        check_rate(&state, user.0.id, false).await?;
    }
    let attachments = files::attachable(&state, user.0.id, &body.attachments, MAX_ATTACHMENTS).await?;
    let post = thread.reply(&state.db, user.0.id, &content, &attachments).await?;
    notify::thread_reply(&state.db, &thread, &post, &user.0).await;
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
    if role == Manager::Moderator && updated.locked && !thread.locked {
        notify::thread_locked(&state.db, &updated, user.0.id).await;
    }
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
    let role = manager(&state, &user, thread.author_id, &thread).await?;
    let attached = File::of_forum_thread(&state.db, thread.id).await?;
    thread.delete(&state.db).await?;
    files::delete_all(&state, attached).await;
    log::info!("user {} deleted forum thread {} ({:?})", user.0.id, thread.id, thread.title);
    if role == Manager::Moderator {
        notify::thread_deleted(&state.db, &thread, user.0.id).await;
    }
    Ok(HttpResponse::NoContent().finish())
}

/// Replaces a post's content and marks it edited. Authors can edit their own (see
/// [`manager`]).
#[patch("/forum/posts/{id}")]
async fn edit_post(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<PostEditBody>,
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
    let role = manager(&state, &user, post.author_id, &thread).await?;
    if post.id == thread.thread_post_id {
        let attached = File::of_forum_thread(&state.db, thread.id).await?;
        thread.delete(&state.db).await?;
        files::delete_all(&state, attached).await;
        log::info!("user {} deleted forum thread {} with its first post", user.0.id, thread.id);
        if role == Manager::Moderator {
            notify::thread_deleted(&state.db, &thread, user.0.id).await;
        }
    } else {
        let attached = File::of_forum_post(&state.db, post.id).await?;
        post.delete_reply(&state.db).await?;
        files::delete_all(&state, attached).await;
        log::info!(
            "user {} deleted forum post {} by user {} in thread {}",
            user.0.id, post.id, post.author_id, thread.id
        );
        if role == Manager::Moderator {
            notify::post_deleted(&state.db, &post, &thread, user.0.id).await;
        }
    }
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
struct VoteBody {
    /// `null` takes the user's vote back.
    vote: Option<Vote>,
}

/// Likes or dislikes a post, or takes the vote back, and returns the post. Each like adds 1
/// to the author's reputation and each dislike takes 1 away. Requires Site Access and User
/// Posts. Users can't vote on their own posts (400). 423 if the thread or its category is
/// locked.
#[put("/forum/posts/{id}/vote")]
async fn vote_post(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<VoteBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, POST).await?;
    let (post, thread) = find_post(&state, id.into_inner()).await?;
    if post.author_id == user.0.id {
        return Err(bad("you can't vote on your own post".to_owned()));
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
    let voted = post.vote(&state.db, user.0.id, body.vote).await?;
    Ok(HttpResponse::Ok().json(voted))
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
/// Moderators (Site Access + Thread Management) can manage anything, locked or not, but not
/// what's written by a user holding permissions they don't (counting the author's groups,
/// punishments aside, as bans do), so a moderator can't edit or delete an admin's posts.
/// Authors can manage their own as long as they can still post (Site Access + User Posts, so
/// not banned) and neither the thread nor its category is locked.
async fn manager(
    state: &AppState,
    user: &AuthUser,
    author_id: i64,
    thread: &ForumThread,
) -> Result<Manager, ApiError> {
    let permissions = Usergroup::permissions_for_user(&state.db, user.0.id).await?;
    if permissions.contains(MODERATE) {
        if author_id != user.0.id {
            let outranking = Usergroup::group_permissions_for_user(&state.db, author_id)
                .await?
                .difference(permissions);
            if !outranking.is_empty() {
                return Err(ApiError::Forbidden(format!(
                    "{} (the author holds permissions you don't)",
                    outranking.names().collect::<Vec<_>>().join(", ")
                )));
            }
        }
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

/// 429 if the user has written [`POST_LIMIT`] posts in the past [`POST_WINDOW_SECS`], or,
/// when `starting_thread`, started [`THREAD_LIMIT`] threads in the past
/// [`THREAD_WINDOW_SECS`]. Callers skip it for moderators.
async fn check_rate(state: &AppState, user_id: i64, starting_thread: bool) -> Result<(), ApiError> {
    if starting_thread {
        let (count, oldest) = ForumThread::recent_by(&state.db, user_id, THREAD_WINDOW_SECS).await?;
        rate_limit(count, oldest, THREAD_LIMIT, THREAD_WINDOW_SECS)?;
    }
    let (count, oldest) = ForumPost::recent_by(&state.db, user_id, POST_WINDOW_SECS).await?;
    rate_limit(count, oldest, POST_LIMIT, POST_WINDOW_SECS)
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

    #[test]
    fn pages() {
        assert_eq!(clamp_page(1, 0, 20), (1, 1));
        assert_eq!(clamp_page(3, 0, 20), (1, 1));
        assert_eq!(clamp_page(1, 20, 20), (1, 1));
        assert_eq!(clamp_page(2, 21, 20), (2, 2));
        assert_eq!(clamp_page(i64::MAX, 41, 20), (3, 3));

        assert_eq!(parse_page(None).ok(), Some(1));
        assert_eq!(parse_page(Some(" ".into())).ok(), Some(1));
        assert_eq!(parse_page(Some("4".into())).ok(), Some(4));
        assert!(parse_page(Some("0".into())).is_err());
        assert!(parse_page(Some("-1".into())).is_err());
        assert!(parse_page(Some("two".into())).is_err());
    }
}
