pub mod community;
pub mod cursor;
pub mod file;
pub mod forum;
pub mod legal;
pub mod loading_screen;
pub mod notification;
pub mod permissions;
pub mod player;
pub mod private_message;
pub mod punishment;
pub mod server;
pub mod store;
pub mod support;
pub mod user;
pub mod usergroup;

pub use community::Community;
pub use file::File;
pub use loading_screen::LoadingScreenConfig;
pub use permissions::Permissions;
pub use player::UserPlayer;
pub use punishment::Punishment;
pub use server::Server;
pub use user::User;
pub use usergroup::Usergroup;

/// `value` with the `LIKE`/`ILIKE` wildcards (`%`, `_`) and the escape character escaped, so
/// a search matches them literally.
pub fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
