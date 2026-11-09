use actix_web::{HttpResponse, get, web};

use crate::{error::ApiError, models::Usergroup, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list);
}

/// Every usergroup's public details (no members or permissions), e.g. for filtering users
/// by group. No login required.
#[get("/usergroups")]
async fn list(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(Usergroup::badges(&state.db).await?))
}
