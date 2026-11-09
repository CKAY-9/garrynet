use actix_web::{HttpResponse, get, web};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{Permissions, Usergroup},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(list_full);
}

/// Every usergroup's public details (no members or permissions), e.g. for filtering users
/// by group. No login required.
#[get("/usergroups")]
async fn list(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(Usergroup::badges(&state.db).await?))
}

/// Every usergroup in full, including members and permissions. Requires Admin Access and
/// Usergroup Management.
#[get("/admin/usergroups")]
async fn list_full(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    user.require(
        &state.db,
        Permissions::ADMIN_ACCESS.union(Permissions::USERGROUP_CONTROL),
    )
    .await?;
    Ok(HttpResponse::Ok().json(Usergroup::list(&state.db).await?))
}
