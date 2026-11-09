use actix_web::{HttpResponse, get, web};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, User, Usergroup,
        user::UserStatus,
        usergroup::UsergroupEffect,
    },
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    // `me` first, so `/users/me` isn't matched as `/users/{id}`.
    cfg.service(me).service(profile);
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

/// A user's public profile: everything but their email, plus the groups they belong to.
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
}

/// What a profile needs to display a usergroup (no member list or permissions).
#[derive(Serialize)]
struct UsergroupBadge {
    id: i64,
    name: String,
    effect: UsergroupEffect,
    color: String,
    icon_url: Option<String>,
}

/// Public profile of the user with the given GarryNet id. No login required.
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
        .map(|g| UsergroupBadge {
            id: g.id,
            name: g.name,
            effect: g.effect,
            color: g.color,
            icon_url: g.icon_url,
        })
        .collect();

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
    }))
}
