use actix_web::{HttpResponse, get, web};
use serde::Serialize;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{Permissions, User},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(stats);
}

/// Statistics for the admin dashboard.
#[derive(Serialize)]
struct Stats {
    users: UserStats,
    // TODO: forum activity (posts over the past week) once the forum exists.
    // TODO: store activity (purchases and income over the past week) once the store exists.
}

#[derive(Serialize)]
struct UserStats {
    registered: i64,
    /// Users currently ONLINE (accessed the site within the last 5 minutes).
    active: i64,
}

/// Dashboard statistics. Requires Admin Access.
#[get("/admin/stats")]
async fn stats(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, Permissions::ADMIN_ACCESS).await?;
    let (registered, active) =
        tokio::try_join!(User::count(&state.db), User::count_online(&state.db))?;
    Ok(HttpResponse::Ok().json(Stats {
        users: UserStats { registered, active },
    }))
}
