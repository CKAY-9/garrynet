use actix_web::{HttpResponse, get, web};
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::{demo, state::AppState};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_demo);
}

#[derive(Serialize)]
struct DemoStatus {
    /// Whether this deployment is a demo (docs/DEMO.md).
    enabled: bool,
    /// When what non-admins created is next cleared; `null` unless `enabled`.
    next_reset: Option<DateTime<Utc>>,
}

/// Whether this is a demo deployment, for the site-wide banner. Public.
#[get("/demo")]
async fn get_demo(state: web::Data<AppState>) -> HttpResponse {
    let enabled = state.config.demo;
    HttpResponse::Ok().json(DemoStatus { enabled, next_reset: enabled.then(|| demo::next_reset(Utc::now())) })
}
