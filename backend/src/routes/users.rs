use actix_web::{HttpResponse, get, web};
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Punishment, User, Usergroup,
        punishment::PunishmentKind,
        user::{LastActivity, UserSearch, UserSort, UserStatus},
        usergroup::UsergroupBadge,
    },
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    // `me` and `search` first, so they aren't matched as `/users/{id}`.
    cfg.service(me).service(search).service(profile);
}

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

/// A ban in effect, as shown to everyone on the profile. Who issued it stays private.
#[derive(Serialize)]
struct PublicBan {
    reason: String,
    created_at: DateTime<Utc>,
    /// `None` for permanent.
    ends: Option<DateTime<Utc>>,
}

/// A user's public profile: everything but their email, plus the groups they belong to and
/// their ban, if they're banned.
#[derive(Serialize)]
struct Profile {
    id: i64,
    username: String,
    avatar_url: String,
    steam_id: String,
    bio: String,
    reputation: i32,
    status: UserStatus,
    joined: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    usergroups: Vec<UsergroupBadge>,
    ban: Option<PublicBan>,
}

/// Public profile of the user with the given GarryNet id, including a ban in effect (reason,
/// date and end, not who issued it). No login required.
#[get("/users/{id}")]
async fn profile(
    state: web::Data<AppState>,
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

    Ok(HttpResponse::Ok().json(Profile {
        status: user.current_status(),
        id: user.id,
        username: user.username,
        avatar_url: user.avatar_url,
        steam_id: user.steam_id,
        bio: user.bio,
        reputation: user.reputation,
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

/// A user as listed in search results: their public profile plus their post count.
#[derive(Serialize)]
struct SearchResult {
    id: i64,
    username: String,
    avatar_url: String,
    reputation: i32,
    status: UserStatus,
    joined: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    post_count: i64,
    usergroups: Vec<UsergroupBadge>,
}

/// Users matching every given filter. No login required.
///
/// - `name`: case-insensitive substring of the username
/// - `id`: GarryNet user id (matches at most one user)
/// - `usergroup`: id of a group the users belong to
/// - `last_activity`: `online`, `hour`, `day`, `week` or `month`: seen within that window
/// - `sort_by`: `reputation+` (most), `reputation-` (least) or `posts` (most). Defaults
///   to best name match, or random users when no name is given.
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
            SearchResult {
                status: user.current_status(),
                usergroups: groups.remove(&user.id).unwrap_or_default(),
                id: user.id,
                username: user.username,
                avatar_url: user.avatar_url,
                reputation: user.reputation,
                joined: user.joined,
                last_seen: user.last_seen,
                post_count: row.post_count,
            }
        })
        .collect();
    Ok(HttpResponse::Ok().json(results))
}

fn parse_search(query: SearchQuery) -> Result<UserSearch, ApiError> {
    /// `None` for a missing or blank value, else the trimmed value.
    fn present(value: Option<String>) -> Option<String> {
        value
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty())
    }
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
