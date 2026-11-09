pub mod admin;
pub mod auth;
pub mod community;
pub mod loading_screen;
pub mod servers;
pub mod users;
pub mod validate;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure)
        .configure(admin::configure)
        .configure(community::configure)
        .configure(loading_screen::configure)
        .configure(servers::configure)
        .configure(users::configure);
}
