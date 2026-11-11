use std::time::{Duration, Instant};

use actix_web::{HttpResponse, get, put, web};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthUser, steam},
    error::ApiError,
    models::{
        LoadingScreenConfig, Permissions, Server, User, UserPlayer,
        loading_screen::LoadingScreenStyle,
    },
    state::AppState,
    steam_profiles::Lookup,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_config).service(update_config).service(for_server);
}

/// The loading screen settings. Public: the `/loading-screen` preview uses them.
#[get("/loading-screen")]
async fn get_config(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(LoadingScreenConfig::get(&state.db).await?))
}

#[derive(Deserialize)]
struct UpdateConfig {
    style: LoadingScreenStyle,
}

/// Changes the loading screen style. Requires Admin Access and Server Management.
#[put("/loading-screen")]
async fn update_config(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<UpdateConfig>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, Permissions::ADMIN_ACCESS | Permissions::SERVER_MANAGEMENT)
        .await?;
    let config = LoadingScreenConfig::set_style(&state.db, body.style).await?;
    log::info!(
        "user {} set the loading screen style to {:?}",
        user.0.id,
        config.style
    );
    Ok(HttpResponse::Ok().json(config))
}

#[derive(Deserialize)]
struct PlayerQuery {
    /// SteamID64 of the player who is loading in (GMod's `%s` in `sv_loadingurl`).
    steam_id: Option<String>,
}

/// Everything one server's loading screen shows.
#[derive(Serialize)]
struct LoadingScreen {
    style: LoadingScreenStyle,
    server: Server,
    /// `None` when no valid SteamID64 was given.
    player: Option<Player>,
}

#[derive(Serialize)]
struct Player {
    steam_id: String,
    /// GarryNet username, else the in-game name, else the Steam name.
    name: String,
    avatar_url: Option<String>,
    /// GarryNet user id, if the player has an account.
    user_id: Option<i64>,
    /// The player's record on this server; `None` on their first visit.
    record: Option<UserPlayer>,
}

/// How long the loading screen waits on Steam for a player without a GarryNet account.
const STEAM_LOOKUP_TIMEOUT: Duration = Duration::from_secs(3);

/// Data for a server's loading screen, for the player with `?steam_id=`. Public: the
/// Garry's Mod client loads the page without logging in.
#[get("/servers/{id}/loading-screen")]
async fn for_server(
    state: web::Data<AppState>,
    id: web::Path<i64>,
    query: web::Query<PlayerQuery>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let server = Server::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("server"))?;
    let config = LoadingScreenConfig::get(&state.db).await?;

    let steam_id = query.into_inner().steam_id.filter(|s| is_steam_id64(s));
    let player = match steam_id {
        Some(steam_id) => Some(find_player(&state, id, steam_id).await?),
        None => None,
    };

    Ok(HttpResponse::Ok().json(LoadingScreen {
        style: config.style,
        server,
        player,
    }))
}

async fn find_player(
    state: &AppState,
    server_id: i64,
    steam_id: String,
) -> Result<Player, ApiError> {
    let (user, record) = tokio::try_join!(
        User::find_by_steam_id(&state.db, &steam_id),
        UserPlayer::find(&state.db, server_id, &steam_id),
    )?;

    if let Some(user) = user {
        return Ok(Player {
            steam_id,
            name: user.username,
            avatar_url: Some(user.avatar_url),
            user_id: Some(user.id),
            record,
        });
    }

    // No GarryNet account, so ask Steam for their name and avatar. The loading screen
    // still works without them, so a slow or failing Steam (or a spent lookup budget, see
    // `SteamProfiles`) only costs the avatar.
    let summary = steam_profile(state, &steam_id).await;

    let name = record
        .as_ref()
        .map(|r| r.ingame_name.clone())
        .or_else(|| summary.as_ref().map(|s| s.personaname.clone()))
        .unwrap_or_else(|| steam_id.clone());
    Ok(Player {
        steam_id,
        name,
        avatar_url: summary.map(|s| s.avatarfull),
        user_id: None,
        record,
    })
}

/// The player's Steam profile, from the cache if possible.
async fn steam_profile(state: &AppState, steam_id: &str) -> Option<steam::PlayerSummary> {
    match state.steam_profiles.lookup(steam_id, Instant::now()) {
        Lookup::Cached(summary) => return summary,
        Lookup::OverBudget => {
            log::debug!("loading screen: skipped the steam lookup for {steam_id}: lookup budget spent");
            return None;
        }
        Lookup::Fetch => {}
    }
    let lookup = steam::player_summary(&state.http, &state.config.steam_api_key, steam_id);
    let result = match tokio::time::timeout(STEAM_LOOKUP_TIMEOUT, lookup).await {
        Ok(Ok(summary)) => Ok(Some(summary)),
        Ok(Err(e)) => {
            log::warn!("loading screen: steam lookup for {steam_id} failed: {e}");
            Err(())
        }
        Err(_) => {
            log::warn!("loading screen: steam lookup for {steam_id} timed out");
            Err(())
        }
    };
    state.steam_profiles.store(steam_id, result.clone(), Instant::now());
    result.ok().flatten()
}

/// A 17-digit SteamID64.
fn is_steam_id64(s: &str) -> bool {
    s.len() == 17 && s.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steam_id64() {
        assert!(is_steam_id64("76561197960287930"));
        assert!(!is_steam_id64("7656119796028793"));
        assert!(!is_steam_id64("STEAM_0:0:11101"));
        assert!(!is_steam_id64(""));
    }
}
