use actix_web::{HttpResponse, get, http::header, post, web};

use crate::{
    auth::{
        auth_cookie, removal_cookie,
        steam::{self, Assertion},
    },
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

/// Starts the login flow by sending the user to Steam.
#[get("/auth/steam")]
async fn steam_login(state: web::Data<AppState>) -> HttpResponse {
    let config = &state.config;
    redirect(&steam::login_url(
        &config.steam_return_to(),
        &config.backend_url,
    ))
}

/// Steam redirects here after the user signs in. Verifies the assertion, creates the
/// GarryNet user on first login, sets the JWT cookie and sends the user to the frontend.
#[get("/auth/steam/callback")]
async fn steam_callback(
    state: web::Data<AppState>,
    query: web::Query<Vec<(String, String)>>,
) -> Result<HttpResponse, ApiError> {
    let config = &state.config;
    let params = query.into_inner();

    let steam_id = match steam::parse_assertion(&params, &config.steam_return_to())? {
        Assertion::Cancelled => return Ok(redirect(&config.frontend_url)),
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
    Ok(res)
}

/// Clears the auth cookie. JWTs are stateless, so this only signs out the current browser.
#[post("/auth/logout")]
async fn logout(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let mut res = redirect(&state.config.frontend_url);
    res.add_cookie(&removal_cookie(&state.config))
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(res)
}
