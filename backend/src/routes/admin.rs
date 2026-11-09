use actix_web::{HttpResponse, get, web};
use serde::Serialize;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{Permissions, User, forum::ForumStats},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(stats);
}

/// Statistics for the admin dashboard.
#[derive(Serialize)]
struct Stats {
    users: UserStats,
    forum: ForumActivity,
    // TODO: store activity (purchases and income over the past week) once the store exists.
}

#[derive(Serialize)]
struct UserStats {
    registered: i64,
    /// Users currently ONLINE (accessed the site within the last 5 minutes).
    active: i64,
}

#[derive(Serialize)]
struct ForumActivity {
    /// Posts (including threads' first posts) made in the past 7 days.
    posts_week: i64,
}

/// Dashboard statistics. Requires Admin Access.
#[get("/admin/stats")]
async fn stats(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, Permissions::ADMIN_ACCESS).await?;
    let (registered, active, posts_week) = tokio::try_join!(
        User::count(&state.db),
        User::count_online(&state.db),
        ForumStats::posts_since_days(&state.db, 7),
    )?;
    Ok(HttpResponse::Ok().json(Stats {
        users: UserStats { registered, active },
        forum: ForumActivity { posts_week },
    }))
}
