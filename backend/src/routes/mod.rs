pub mod admin;
pub mod auth;
pub mod community;
pub mod files;
pub mod forum;
pub mod gnet;
pub mod loading_screen;
pub mod markdown;
pub mod notifications;
pub mod punishments;
pub mod servers;
pub mod store;
pub mod store_admin;
pub mod support;
pub mod usergroups;
pub mod users;
pub mod validate;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.configure(auth::configure)
        .configure(admin::configure)
        .configure(community::configure)
        .configure(files::configure)
        .configure(forum::configure)
        .configure(gnet::configure)
        .configure(loading_screen::configure)
        .configure(notifications::configure)
        .configure(punishments::configure)
        .configure(servers::configure)
        .configure(store::configure)
        .configure(store_admin::configure)
        .configure(support::configure)
        .configure(usergroups::configure)
        .configure(users::configure);
}
