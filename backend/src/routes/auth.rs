use actix_web::{HttpRequest, HttpResponse, get, http::header, post, web};

use crate::{
    auth::{
        AuthUser, LOGIN_STATE_COOKIE, auth_cookie, login_state_cookie, login_state_removal_cookie, removal_cookie,
        steam::{self, Assertion},
    },
    config::Config,
    error::ApiError,
    models::User,
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(steam_login)
        .service(steam_callback)
        .service(logout);
}

fn redirect(to: &str) -> HttpResponse {
    HttpResponse::SeeOther()
        .insert_header((header::LOCATION, to))
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .finish()
}

/// Starts the login flow by sending the user to Steam. A random nonce goes both into a
/// cookie and into the `return_to` URL Steam signs, so the callback only accepts an
/// assertion from a login this browser started: otherwise an attacker could send a victim to
/// the callback with an assertion for the attacker's own account (login CSRF).
#[get("/auth/steam")]
async fn steam_login(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let config = &state.config;
    let nonce = new_nonce()?;
    let mut res = redirect(&steam::login_url(&config.steam_return_to(&nonce), &config.backend_url));
    res.add_cookie(&login_state_cookie(config, nonce))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(res)
}

/// Steam redirects here after the user signs in. Verifies the assertion (including that its
/// `return_to` carries this browser's login nonce), creates the GarryNet user on first
/// login, sets the JWT cookie and sends the user to the frontend.
#[get("/auth/steam/callback")]
async fn steam_callback(
    state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<Vec<(String, String)>>,
) -> Result<HttpResponse, ApiError> {
    let config = &state.config;
    let params = query.into_inner();

    let nonce = req
        .cookie(LOGIN_STATE_COOKIE)
        .map(|c| c.value().to_owned())
        .filter(|n| n.len() == NONCE_BYTES * 2 && n.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| {
            ApiError::SteamVerification("this login wasn't started from this browser, or took too long; log in again".into())
        })?;
    let steam_id = match steam::parse_assertion(&params, &config.steam_return_to(&nonce))? {
        Assertion::Cancelled => return finish(redirect(&config.frontend_url), config),
        Assertion::Unverified { steam_id } => steam_id,
    };
    steam::verify_with_steam(&state.http, &params).await?;

    let profile = steam::player_summary(&state.http, &config.steam_api_key, &steam_id).await?;
    let user =
        User::upsert_from_steam(&state.db, &steam_id, &profile.personaname, &profile.avatarfull)
            .await?;
    let token = state.jwt.issue(&user)?;

    log::info!("user {} ({}) logged in", user.id, user.steam_id);

    let mut res = redirect(&config.frontend_url);
    res.add_cookie(&auth_cookie(config, token))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    finish(res, config)
}

const NONCE_BYTES: usize = 16;

/// 16 random bytes in hex.
fn new_nonce() -> Result<String, ApiError> {
    let mut bytes = [0u8; NONCE_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::Internal(format!("getrandom: {e}")))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Clears the login-state cookie: each nonce is good for one login.
fn finish(mut res: HttpResponse, config: &Config) -> Result<HttpResponse, ApiError> {
    res.add_cookie(&login_state_removal_cookie(config))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(res)
}

/// Signs the user out everywhere: revokes every token issued to them (so a stolen one stops
/// working too) and clears this browser's auth cookie. Without a valid token it only clears
/// the cookie.
#[post("/auth/logout")]
async fn logout(state: web::Data<AppState>, user: Option<AuthUser>) -> Result<HttpResponse, ApiError> {
    if let Some(AuthUser(user)) = user {
        User::revoke_tokens(&state.db, user.id).await?;
        log::info!("user {} logged out; their tokens are revoked", user.id);
    }
    let mut res = redirect(&state.config.frontend_url);
    res.add_cookie(&removal_cookie(&state.config))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(res)
}
