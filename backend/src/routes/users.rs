use actix_web::{HttpResponse, get, patch, web};
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthUser, MaybeUser},
    error::ApiError,
    models::{
        Permissions, Punishment, User, Usergroup,
        forum::{Authored, AuthoredFilter, AuthoredKind, AuthoredSort, ForumPost},
        punishment::PunishmentKind,
        user::{LastActivity, UserSearch, UserSort, UserStatus},
        usergroup::UsergroupBadge,
    },
    routes::validate::{bad, present, text},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    // `me` and `search` first, so they aren't matched as `/users/{id}`.
    cfg.service(me)
        .service(update_me)
        .service(search)
        .service(profile)
        .service(posts);
}

/// Who sees a private profile in full besides its owner: staff who manage users, the same
/// permissions as banning (see `routes::punishments`).
const SEE_PRIVATE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::USER_MANAGEMENT);

/// Must match `MAX_BIO` in the frontend (src/app/(site)/settings/SettingsForm.tsx) and the
/// `users_bio_length` check.
const MAX_BIO: usize = 1000;

/// The current user plus their effective permissions, so the frontend can decide which
/// links and pages to show. The backend still checks permissions on every protected route.
#[derive(Serialize)]
struct Me {
    #[serde(flatten)]
    user: User,
    permissions: Permissions,
}

/// The currently authenticated user.
#[get("/users/me")]
async fn me(
    state: web::Data<AppState>,
    AuthUser(user): AuthUser,
) -> Result<HttpResponse, ApiError> {
    let permissions = Usergroup::permissions_for_user(&state.db, user.id).await?;
    Ok(HttpResponse::Ok().json(Me { user, permissions }))
}

#[derive(Deserialize)]
struct ProfileSettings {
    bio: Option<String>,
    private: Option<bool>,
}

/// Changes the current user's bio and whether their profile is private; omitted fields stay
/// as they are. Returns the user like `GET /users/me`. Requires Site Access, so banned
/// users can't (suspended users can).
#[patch("/users/me")]
async fn update_me(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<ProfileSettings>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let bio = body.bio.map(|bio| text("bio", &bio, MAX_BIO)).transpose()?;
    if bio.is_none() && body.private.is_none() {
        return Err(bad("nothing to change".to_owned()));
    }
    let permissions = user.require(&state.db, Permissions::SITE_ACCESS).await?;
    let updated = User::update_profile(&state.db, user.0.id, bio.as_deref(), body.private).await?;
    if updated.private != user.0.private {
        log::info!("user {} made their profile {}", updated.id, if updated.private { "private" } else { "public" });
    }
    Ok(HttpResponse::Ok().json(Me { user: updated, permissions }))
}

/// Whether `viewer` sees `user`'s private details (bio, reputation, threads and posts):
/// always for a public profile, else only the user themselves and staff.
async fn sees_private(state: &AppState, viewer: &MaybeUser, user: &User) -> Result<bool, ApiError> {
    Ok(match &viewer.0 {
        _ if !user.private => true,
        None => false,
        Some(AuthUser(viewer)) if viewer.id == user.id => true,
        Some(AuthUser(viewer)) => Usergroup::permissions_for_user(&state.db, viewer.id)
            .await?
            .contains(SEE_PRIVATE),
    })
}

/// A ban in effect, as shown to everyone on the profile. Who issued it stays private.
#[derive(Serialize)]
struct PublicBan {
    reason: String,
    created_at: DateTime<Utc>,
    /// `None` for permanent.
    ends: Option<DateTime<Utc>>,
}

/// A user's public profile: everything but their email, plus the groups they belong to and
/// their ban, if they're banned. A private profile's bio and reputation are `None` unless
/// the viewer may see them (see [`sees_private`]).
#[derive(Serialize)]
struct Profile {
    id: i64,
    username: String,
    avatar_url: String,
    steam_id: String,
    bio: Option<String>,
    reputation: Option<i32>,
    private: bool,
    status: UserStatus,
    joined: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    usergroups: Vec<UsergroupBadge>,
    ban: Option<PublicBan>,
}

/// Public profile of the user with the given GarryNet id, including a ban in effect (reason,
/// date and end, not who issued it). No login required; a private profile's bio and
/// reputation are only included for the user themselves and staff.
#[get("/users/{id}")]
async fn profile(
    state: web::Data<AppState>,
    viewer: MaybeUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let user = User::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("user"))?;
    let usergroups = Usergroup::for_user(&state.db, id)
        .await?
        .into_iter()
        .map(UsergroupBadge::from)
        .collect();
    let ban = Punishment::in_effect_for(&state.db, id)
        .await?
        .into_iter()
        .find(|p| p.kind == PunishmentKind::Ban)
        .map(|p| PublicBan {
            reason: p.reason,
            created_at: p.created_at,
            ends: p.ends,
        });
    let full = sees_private(&state, &viewer, &user).await?;

    Ok(HttpResponse::Ok().json(Profile {
        status: user.current_status(),
        id: user.id,
        username: user.username,
        avatar_url: user.avatar_url,
        steam_id: user.steam_id,
        bio: full.then_some(user.bio),
        reputation: full.then_some(user.reputation),
        private: user.private,
        joined: user.joined,
        last_seen: user.last_seen,
        usergroups,
        ban,
    }))
}

/// Results returned when the request doesn't ask for a specific number.
const DEFAULT_SEARCH_LIMIT: i64 = 24;
const MAX_SEARCH_LIMIT: i64 = 100;

/// Query string of `GET /users/search`. Every field is optional, and empty values are
/// ignored, so a plain HTML form can submit all its fields.
#[derive(Deserialize)]
struct SearchQuery {
    name: Option<String>,
    id: Option<String>,
    usergroup: Option<String>,
    last_activity: Option<String>,
    sort_by: Option<String>,
    limit: Option<String>,
}

/// A user as listed in search results: their public profile plus their post count. Private
/// users' reputation and post count are `None`.
#[derive(Serialize)]
struct SearchResult {
    id: i64,
    username: String,
    avatar_url: String,
    reputation: Option<i32>,
    status: UserStatus,
    joined: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    post_count: Option<i64>,
    usergroups: Vec<UsergroupBadge>,
}

/// Users matching every given filter. No login required.
///
/// - `name`: case-insensitive substring of the username
/// - `id`: GarryNet user id (matches at most one user)
/// - `usergroup`: id of a group the users belong to
/// - `last_activity`: `online`, `hour`, `day`, `week` or `month`: seen within that window
/// - `sort_by`: `reputation+` (most), `reputation-` (least) or `posts` (most). Defaults
///   to best name match, or random users when no name is given. Private users, whose
///   reputation and post count are hidden, sort last.
/// - `limit`: 1 to 100, default 24
#[get("/users/search")]
async fn search(
    state: web::Data<AppState>,
    query: web::Query<SearchQuery>,
) -> Result<HttpResponse, ApiError> {
    let search = parse_search(query.into_inner())?;
    let rows = User::search(&state.db, &search).await?;

    let ids: Vec<i64> = rows.iter().map(|r| r.user.id).collect();
    let mut groups: HashMap<i64, Vec<UsergroupBadge>> = HashMap::new();
    for member in Usergroup::badges_for_users(&state.db, &ids).await? {
        groups.entry(member.user_id).or_default().push(member.badge);
    }

    let results: Vec<SearchResult> = rows
        .into_iter()
        .map(|row| {
            let user = row.user;
            let public = !user.private;
            SearchResult {
                status: user.current_status(),
                usergroups: groups.remove(&user.id).unwrap_or_default(),
                id: user.id,
                username: user.username,
                avatar_url: user.avatar_url,
                reputation: public.then_some(user.reputation),
                joined: user.joined,
                last_seen: user.last_seen,
                post_count: public.then_some(row.post_count),
            }
        })
        .collect();
    Ok(HttpResponse::Ok().json(results))
}

fn parse_search(query: SearchQuery) -> Result<UserSearch, ApiError> {
    fn id(field: &str, value: Option<String>) -> Result<Option<i64>, ApiError> {
        present(value)
            .map(|v| {
                v.parse::<i64>()
                    .map_err(|_| ApiError::BadRequest(format!("{field} must be a number")))
            })
            .transpose()
    }

    let last_activity = present(query.last_activity)
        .map(|v| match v.as_str() {
            "online" => Ok(LastActivity::Online),
            "hour" => Ok(LastActivity::Hour),
            "day" => Ok(LastActivity::Day),
            "week" => Ok(LastActivity::Week),
            "month" => Ok(LastActivity::Month),
            _ => Err(ApiError::BadRequest(
                "last_activity must be one of online, hour, day, week, month".to_owned(),
            )),
        })
        .transpose()?;

    // An unencoded `+` in a query string decodes to a space, which `present` trims, so
    // `reputation` on its own means `reputation+`.
    let sort = match present(query.sort_by).as_deref() {
        None => UserSort::BestMatch,
        Some("reputation+" | "reputation") => UserSort::ReputationDesc,
        Some("reputation-") => UserSort::ReputationAsc,
        Some("posts") => UserSort::Posts,
        Some(_) => {
            return Err(ApiError::BadRequest(
                "sort_by must be one of reputation+, reputation-, posts".to_owned(),
            ));
        }
    };

    let limit = id("limit", query.limit)?.unwrap_or(DEFAULT_SEARCH_LIMIT);
    if !(1..=MAX_SEARCH_LIMIT).contains(&limit) {
        return Err(ApiError::BadRequest(format!(
            "limit must be between 1 and {MAX_SEARCH_LIMIT}"
        )));
    }

    Ok(UserSearch {
        name: present(query.name),
        id: id("id", query.id)?,
        usergroup: id("usergroup", query.usergroup)?,
        last_activity,
        sort,
        limit,
    })
}

/// Results per page when the request doesn't ask for a specific number.
const DEFAULT_POSTS_LIMIT: i64 = 20;
const MAX_POSTS_LIMIT: i64 = 50;
const MAX_POSTS_SEARCH: usize = 100;

/// Query string of `GET /users/{id}/posts`. Like search, empty values are ignored.
#[derive(Deserialize)]
struct PostsQuery {
    kind: Option<String>,
    q: Option<String>,
    sort: Option<String>,
    limit: Option<String>,
    offset: Option<String>,
}

#[derive(Serialize)]
struct PostsPage {
    items: Vec<Authored>,
    /// More match after these.
    has_more: bool,
}

/// The threads and replies a user has written, for their profile. No login required, but a
/// private user's are only listed for themselves and staff (403 for anyone else).
///
/// - `kind`: `thread` (threads they started) or `post` (their replies); both by default
/// - `q`: case-insensitive part of the thread title or post content
/// - `sort`: `new` (default), `old`, `reputation+` (most likes minus dislikes) or
///   `reputation-`
/// - `limit`: 1 to 50, default 20; `offset`: how many to skip
#[get("/users/{id}/posts")]
async fn posts(
    state: web::Data<AppState>,
    viewer: MaybeUser,
    id: web::Path<i64>,
    query: web::Query<PostsQuery>,
) -> Result<HttpResponse, ApiError> {
    let filter = parse_posts(query.into_inner())?;
    let user = User::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("user"))?;
    if !sees_private(&state, &viewer, &user).await? {
        let names: Vec<&str> = SEE_PRIVATE.names().collect();
        return Err(ApiError::Forbidden(format!("{} (the profile is private)", names.join(", "))));
    }

    // One extra tells whether there's another page.
    let mut items = ForumPost::by_author(
        &state.db,
        user.id,
        &AuthoredFilter { limit: filter.limit + 1, ..filter },
    )
    .await?;
    let has_more = items.len() as i64 > filter.limit;
    items.truncate(filter.limit as usize);
    Ok(HttpResponse::Ok().json(PostsPage { items, has_more }))
}

fn parse_posts(query: PostsQuery) -> Result<AuthoredFilter, ApiError> {
    fn number(field: &str, value: Option<String>) -> Result<Option<i64>, ApiError> {
        present(value)
            .map(|v| v.parse::<i64>().map_err(|_| bad(format!("{field} must be a number"))))
            .transpose()
    }

    let kind = match present(query.kind).as_deref() {
        None => None,
        Some("thread") => Some(AuthoredKind::Threads),
        Some("post") => Some(AuthoredKind::Replies),
        Some(_) => return Err(bad("kind must be one of thread, post".to_owned())),
    };
    // As in search, an unencoded `+` arrives as a space, which `present` trims.
    let sort = match present(query.sort).as_deref() {
        None | Some("new") => AuthoredSort::Newest,
        Some("old") => AuthoredSort::Oldest,
        Some("reputation+" | "reputation") => AuthoredSort::ReputationDesc,
        Some("reputation-") => AuthoredSort::ReputationAsc,
        Some(_) => {
            return Err(bad("sort must be one of new, old, reputation+, reputation-".to_owned()));
        }
    };
    let q = present(query.q).map(|q| text("q", &q, MAX_POSTS_SEARCH)).transpose()?;

    let limit = number("limit", query.limit)?.unwrap_or(DEFAULT_POSTS_LIMIT);
    if !(1..=MAX_POSTS_LIMIT).contains(&limit) {
        return Err(bad(format!("limit must be between 1 and {MAX_POSTS_LIMIT}")));
    }
    let offset = number("offset", query.offset)?.unwrap_or(0);
    if offset < 0 {
        return Err(bad("offset must be 0 or greater".to_owned()));
    }

    Ok(AuthoredFilter { kind, search: q, sort, limit, offset })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> PostsQuery {
        PostsQuery { kind: None, q: None, sort: None, limit: None, offset: None }
    }

    #[test]
    fn posts_defaults() {
        let filter = parse_posts(query()).unwrap_or_else(|_| panic!("empty query rejected"));
        assert_eq!(filter.kind, None);
        assert_eq!(filter.sort, AuthoredSort::Newest);
        assert_eq!(filter.limit, DEFAULT_POSTS_LIMIT);
        assert_eq!(filter.offset, 0);
        assert!(filter.search.is_none());
    }

    #[test]
    fn posts_filters() {
        let filter = parse_posts(PostsQuery {
            kind: Some("thread".into()),
            q: Some("  rules ".into()),
            // `reputation+` sent with an unencoded `+`.
            sort: Some("reputation ".into()),
            limit: Some("".into()),
            offset: Some("40".into()),
        })
        .unwrap_or_else(|_| panic!("valid query rejected"));
        assert_eq!(filter.kind, Some(AuthoredKind::Threads));
        assert_eq!(filter.search.as_deref(), Some("rules"));
        assert_eq!(filter.sort, AuthoredSort::ReputationDesc);
        assert_eq!(filter.offset, 40);

        assert!(parse_posts(PostsQuery { kind: Some("reply".into()), ..query() }).is_err());
        assert!(parse_posts(PostsQuery { sort: Some("likes".into()), ..query() }).is_err());
        assert!(parse_posts(PostsQuery { limit: Some("51".into()), ..query() }).is_err());
        assert!(parse_posts(PostsQuery { offset: Some("-1".into()), ..query() }).is_err());
        assert!(parse_posts(PostsQuery { q: Some("x".repeat(101)), ..query() }).is_err());
    }
}
