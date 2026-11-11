//! Routes used by the GarryNet Garry's Mod addon (docs/ADDON.md). A server links once with
//! its access key, then authenticates with the JWT it gets back.

use actix_web::{HttpRequest, HttpResponse, get, post, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    auth::{AuthServer, authorization},
    error::ApiError,
    models::{
        Server, UserPlayer,
        player::PlayerSession,
        store::{AckResult, Fulfillment, FulfillmentStatus, MAX_ATTEMPTS},
    },
    notify,
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
            .service(unlink)
            .service(fulfillments)
            .service(acknowledge_fulfillment),
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
/// Gamemode stats per player per heartbeat, of each kind (values and counters).
const MAX_STATS: usize = 32;
const MAX_STAT_KEY: usize = 64;
const MAX_STAT_TEXT: usize = 128;
/// Most a counter can grow by in one heartbeat.
const MAX_COUNTER: f64 = 1_000_000.0;

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
    /// Latest gamemode stats, e.g. `{"darkrp_money": 5000}`. Any JSON, since GMod encodes
    /// an empty table as `[]`; see `stat_values`.
    #[serde(default)]
    stats: Value,
    /// Gamemode counters since the last heartbeat that reported this player, e.g.
    /// `{"sandbox_props_spawned": 3}`; see `stat_counters`.
    #[serde(default)]
    counters: Value,
}

#[derive(Serialize)]
struct HeartbeatResponse {
    server: Server,
    /// A replacement JWT, when the current one is due for rotation.
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

/// The per-minute sync: the server's gamemode and map, and its players. The first one after
/// linking is the server's first heartbeat. A new gamemode clears every player's stats first.
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
    // Before the new gamemode is stored, so the old one can be compared.
    let cleared = UserPlayer::clear_stats_if_gamemode_changed(&mut tx, auth.id, &gamemode).await?;
    UserPlayer::record_sessions(&mut tx, auth.id, &sessions).await?;
    let server = Server::heartbeat(&mut tx, auth.id, &gamemode, &map)
        .await?
        .ok_or(ApiError::Unauthorized)?;
    tx.commit().await?;
    if cleared > 0 {
        log::info!(
            "server {} changed gamemode to {gamemode:?}: cleared the stats of {cleared} player(s)",
            auth.id
        );
    }

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
                stats: Value::Object(stat_values(p.stats)),
                counters: Value::Object(stat_counters(p.counters)),
            })
        })
        .collect()
}

fn truncate(value: &str, max: usize) -> String {
    value.trim().chars().take(max).collect()
}

/// Stat keys are lowercase `gamemode_name` identifiers, e.g. `darkrp_money`.
fn stat_key(key: &str) -> bool {
    (1..=MAX_STAT_KEY).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The first `MAX_STATS` valid entries of a stats object; anything else (including a
/// non-object) is dropped rather than failing the heartbeat, since a value can come from a
/// player (a custom job title).
fn stat_entries(value: Value) -> impl Iterator<Item = (String, Value)> {
    let map = match value {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    map.into_iter().filter(|(k, _)| stat_key(k)).take(MAX_STATS)
}

/// Integral floats become integers: GMod's JSON encoder may write `5000` as `5000.0`.
fn number(n: f64) -> Option<Value> {
    if !n.is_finite() {
        None
    } else if n.fract() == 0.0 && n.abs() < 9e15 {
        Some((n as i64).into())
    } else {
        serde_json::Number::from_f64(n).map(Value::Number)
    }
}

/// Reported stat values: numbers, booleans, and strings (trimmed, truncated; empty dropped).
fn stat_values(value: Value) -> Map<String, Value> {
    stat_entries(value)
        .filter_map(|(k, v)| {
            let v = match v {
                Value::Number(n) => number(n.as_f64()?)?,
                Value::Bool(b) => Value::Bool(b),
                Value::String(s) => match truncate(&s, MAX_STAT_TEXT) {
                    s if s.is_empty() => return None,
                    s => Value::String(s),
                },
                _ => return None,
            };
            Some((k, v))
        })
        .collect()
}

/// Reported counter increments: whole, positive, at most `MAX_COUNTER`.
fn stat_counters(value: Value) -> Map<String, Value> {
    stat_entries(value)
        .filter_map(|(k, v)| {
            let n = v.as_f64()?.floor();
            (n.is_finite() && n >= 1.0).then(|| (k, (n.min(MAX_COUNTER) as i64).into()))
        })
        .collect()
}

#[derive(Deserialize)]
struct FulfillmentQuery {
    /// Only this player's fulfillments (e.g. when they join).
    steam_id: Option<String>,
}

/// Store fulfillments the server has to act on (docs/STORE.md): pending ones to deliver,
/// failed ones to retry, and revoked ones whose rewards it delivered and must take back.
/// Oldest first. `?steam_id=` narrows it to one player.
#[get("/{id}/fulfillments")]
async fn fulfillments(
    state: web::Data<AppState>,
    auth: AuthServer,
    id: web::Path<i64>,
    query: web::Query<FulfillmentQuery>,
) -> Result<HttpResponse, ApiError> {
    auth.require_id(id.into_inner())?;
    let steam_id = query.into_inner().steam_id.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty());
    let work = Fulfillment::work_for_server(&state.db, auth.id, steam_id.as_deref()).await?;
    Ok(HttpResponse::Ok().json(work))
}

#[derive(Deserialize)]
struct AckBody {
    result: AckResult,
    /// What went wrong, for `failed`.
    #[serde(default)]
    error: String,
}

const MAX_ERROR: usize = 1000;

/// The addon reports what it did with a fulfillment: `fulfilled`, `failed` (with `error`;
/// retried later) or `revoked` (the rewards of a revoked fulfillment were taken back).
/// Idempotent: repeating a report changes nothing. Returns the fulfillment as it stands.
#[post("/{id}/fulfillments/{fulfillment_id}/ack")]
async fn acknowledge_fulfillment(
    state: web::Data<AppState>,
    auth: AuthServer,
    path: web::Path<(i64, i64)>,
    body: web::Json<AckBody>,
) -> Result<HttpResponse, ApiError> {
    let (id, fulfillment_id) = path.into_inner();
    auth.require_id(id)?;
    let body = body.into_inner();
    let error = truncate(&body.error, MAX_ERROR);
    let error = (body.result == AckResult::Failed).then_some(if error.is_empty() { "unknown error" } else { error.as_str() });
    let (fulfillment, changed) = Fulfillment::acknowledge(&state.db, auth.id, fulfillment_id, body.result, error)
        .await?
        .ok_or(ApiError::NotFound("fulfillment"))?;
    match body.result {
        AckResult::Fulfilled => log::info!("server {} fulfilled store purchase {fulfillment_id} for {}", auth.id, fulfillment.steam_id),
        AckResult::Failed => log::warn!(
            "server {} failed to fulfill store purchase {fulfillment_id} (attempt {}): {}",
            auth.id, fulfillment.attempts, fulfillment.last_error.as_deref().unwrap_or("")
        ),
        AckResult::Revoked => log::info!("server {} revoked the rewards of store purchase {fulfillment_id}", auth.id),
    }
    // A repeated report changes nothing, so it doesn't notify again.
    if changed {
        match body.result {
            // Rewards delivered after a revocation are about to be taken back.
            AckResult::Fulfilled if fulfillment.status == FulfillmentStatus::Fulfilled => {
                notify::fulfillment_delivered(&state.db, &fulfillment).await;
            }
            // The addon isn't offered the fulfillment again after this attempt.
            AckResult::Failed if fulfillment.attempts == MAX_ATTEMPTS => {
                notify::fulfillment_failed(&state.db, &fulfillment).await;
            }
            _ => {}
        }
    }
    Ok(HttpResponse::Ok().json(fulfillment))
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
            stats: Value::Array(vec![]),
            counters: Value::Null,
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
    fn normalizes_stats() {
        let mut p = player("76561197960287930");
        p.stats = serde_json::json!({
            "darkrp_money": 5000.0,
            "darkrp_rate": 1.5,
            "darkrp_job": "  Gun Dealer  ",
            "darkrp_wanted": false,
            "darkrp_empty": " ",
            "darkrp_nested": { "a": 1 },
            "darkrp_null": null,
            "Bad Key": 1,
        });
        p.counters = serde_json::json!({
            "sandbox_props_spawned": 3.0,
            "sandbox_tools_used": 2.7,
            "sandbox_zero": 0,
            "sandbox_negative": -5,
            "sandbox_text": "7",
            "sandbox_huge": 1e300,
        });
        let s = sessions(vec![p], Utc::now()).unwrap_or_else(|_| panic!("valid player rejected"));
        assert_eq!(
            s[0].stats,
            serde_json::json!({
                "darkrp_money": 5000,
                "darkrp_rate": 1.5,
                "darkrp_job": "Gun Dealer",
                "darkrp_wanted": false,
            })
        );
        assert_eq!(
            s[0].counters,
            serde_json::json!({
                "sandbox_props_spawned": 3,
                "sandbox_tools_used": 2,
                "sandbox_huge": MAX_COUNTER as i64,
            })
        );
        // GMod's empty table (`[]`) and a missing field are both no stats.
        let s = sessions(vec![player("76561197960287930")], Utc::now())
            .unwrap_or_else(|_| panic!("valid player rejected"));
        assert_eq!(s[0].stats, serde_json::json!({}));
        assert_eq!(s[0].counters, serde_json::json!({}));
    }

    #[test]
    fn rejects_bad_players() {
        assert!(sessions(vec![player("STEAM_0:1:1")], Utc::now()).is_err());
        let many = (0..=MAX_PLAYERS).map(|_| player("76561197960287930")).collect();
        assert!(sessions(many, Utc::now()).is_err());
    }
}
