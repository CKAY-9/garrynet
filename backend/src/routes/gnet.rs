//! Routes used by the GarryNet Garry's Mod addon (docs/ADDON.md). A server links once with
//! its access key, then authenticates with the JWT it gets back.

use actix_web::{HttpRequest, HttpResponse, post, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{AuthServer, authorization},
    error::ApiError,
    models::{Server, UserPlayer, player::PlayerSession},
    routes::validate::{bad, text},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/gnet/server")
            // A full server's heartbeat is larger than the default 32 KiB limit allows.
            .app_data(web::JsonConfig::default().limit(256 * 1024))
            .service(link)
            .service(heartbeat)
            .service(unlink),
    );
}

#[derive(Serialize)]
struct Linked {
    server_id: i64,
    /// The server's JWT. Rolling: later responses may carry a replacement.
    token: String,
    server: Server,
}

/// Exchanges a server's access key (the `Authorization` header, with or without `Bearer `)
/// for a JWT. The key is single-use: it's revoked here.
#[post("/link")]
async fn link(state: web::Data<AppState>, req: HttpRequest) -> Result<HttpResponse, ApiError> {
    let key = authorization(&req).ok_or(ApiError::Unauthorized)?;
    let (server, version) = Server::link(&state.db, &key)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    let token = state.jwt.issue_server(server.id, version)?;
    log::info!("server {} ({:?}) linked", server.id, server.name);
    Ok(HttpResponse::Ok().json(Linked {
        server_id: server.id,
        token,
        server,
    }))
}

const MAX_GAMEMODE: usize = 64;
const MAX_MAP: usize = 128;
const MAX_PLAYERS: usize = 256;
const MAX_PLAYER_NAME: usize = 64;
const MAX_USERGROUP: usize = 64;
/// Playtime piles up on the addon while GarryNet is unreachable; cap what one heartbeat can
/// add at a week.
const MAX_MINUTES: i32 = 7 * 24 * 60;

#[derive(Deserialize)]
struct HeartbeatBody {
    #[serde(default)]
    gamemode: String,
    #[serde(default)]
    map: String,
    #[serde(default)]
    players: Vec<PlayerBody>,
}

#[derive(Deserialize)]
struct PlayerBody {
    steam_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    usergroup: String,
    /// Unix seconds. Numbers are `f64` because GMod's JSON encoder may write integers as
    /// floats.
    connected_at: Option<f64>,
    /// Playtime since the last heartbeat that reported this player.
    #[serde(default)]
    minutes: f64,
}

#[derive(Serialize)]
struct HeartbeatResponse {
    server: Server,
    /// A replacement JWT, when the current one is due for rotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

/// The per-minute sync: the server's gamemode and map, and its players. The first one after
/// linking is the server's first heartbeat.
#[post("/{id}/heartbeat")]
async fn heartbeat(
    state: web::Data<AppState>,
    auth: AuthServer,
    id: web::Path<i64>,
    body: web::Json<HeartbeatBody>,
) -> Result<HttpResponse, ApiError> {
    auth.require_id(id.into_inner())?;
    let body = body.into_inner();
    let gamemode = text("gamemode", &body.gamemode, MAX_GAMEMODE)?;
    let map = text("map", &body.map, MAX_MAP)?;
    let sessions = sessions(body.players, Utc::now())?;

    let mut tx = state.db.begin().await?;
    UserPlayer::record_sessions(&mut tx, auth.id, &sessions).await?;
    let server = Server::heartbeat(&mut tx, auth.id, &gamemode, &map)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(HeartbeatResponse {
        server,
        token: auth.rotated_token(&state.jwt)?,
    }))
}

/// Validates the reported players. Names and usergroups come from players, so they're
/// truncated rather than failing the whole heartbeat.
fn sessions(players: Vec<PlayerBody>, now: DateTime<Utc>) -> Result<Vec<PlayerSession>, ApiError> {
    if players.len() > MAX_PLAYERS {
        return Err(bad(format!("at most {MAX_PLAYERS} players per heartbeat")));
    }
    players
        .into_iter()
        .map(|p| {
            let steam_id = p.steam_id.trim();
            if steam_id.len() != 17 || !steam_id.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad(format!("invalid steam_id {steam_id:?}")));
            }
            let connected_at = p
                .connected_at
                .and_then(|t| DateTime::from_timestamp(t as i64, 0))
                .map_or(now, |t| t.min(now));
            Ok(PlayerSession {
                steam_id: steam_id.to_owned(),
                name: truncate(&p.name, MAX_PLAYER_NAME),
                usergroup: match truncate(&p.usergroup, MAX_USERGROUP) {
                    g if g.is_empty() => "user".to_owned(),
                    g => g,
                },
                connected_at,
                minutes: (p.minutes as i32).clamp(0, MAX_MINUTES),
            })
        })
        .collect()
}

fn truncate(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

/// Unlinks the server at the addon's request, revoking its token.
#[post("/{id}/unlink")]
async fn unlink(
    state: web::Data<AppState>,
    auth: AuthServer,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    auth.require_id(id.into_inner())?;
    Server::unlink(&state.db, auth.id).await?;
    log::info!("server {} unlinked itself", auth.id);
    Ok(HttpResponse::NoContent().finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(steam_id: &str) -> PlayerBody {
        PlayerBody {
            steam_id: steam_id.into(),
            name: "  Garry  ".into(),
            usergroup: String::new(),
            connected_at: Some(0.0),
            minutes: 2.0,
        }
    }

    #[test]
    fn normalizes_players() {
        let now = Utc::now();
        let mut future = player("76561197960287930");
        future.connected_at = Some((now.timestamp() + 3600) as f64);
        future.minutes = 1e12;
        let s = sessions(vec![player("76561197960287930"), future], now)
            .unwrap_or_else(|_| panic!("valid players rejected"));
        assert_eq!(s[0].name, "Garry");
        assert_eq!(s[0].usergroup, "user");
        assert_eq!(s[0].connected_at.timestamp(), 0);
        assert_eq!(s[0].minutes, 2);
        assert_eq!(s[1].connected_at, now);
        assert_eq!(s[1].minutes, MAX_MINUTES);
    }

    #[test]
    fn rejects_bad_players() {
        assert!(sessions(vec![player("STEAM_0:1:1")], Utc::now()).is_err());
        let many = (0..=MAX_PLAYERS).map(|_| player("76561197960287930")).collect();
        assert!(sessions(many, Utc::now()).is_err());
    }
}
