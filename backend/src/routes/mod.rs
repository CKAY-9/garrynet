pub mod admin;
pub mod auth;
pub mod users;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure)
        .configure(admin::configure).configure(users::configure);
}
