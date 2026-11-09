pub mod jwt;
pub mod steam;

use std::{future::Future, pin::Pin};

use actix_web::{
    FromRequest, HttpRequest,
    cookie::{Cookie, SameSite, time},
    dev::Payload,
    http::header,
    web,
};

use sqlx::PgPool;

use crate::{
    config::Config,
    error::ApiError,
    models::{Permissions, User, Usergroup},
    state::AppState,
};

/// Name of the httpOnly cookie holding the user's JWT. The frontend reads it by this name.
pub const AUTH_COOKIE: &str = "garrynet_token";

/// Builds the auth cookie. It's readable by the frontend's Next.js server (same host, or a
/// sibling subdomain when `COOKIE_DOMAIN` is set) but not by page JavaScript.
pub fn auth_cookie(config: &Config, token: String) -> Cookie<'static> {
    let mut cookie = Cookie::build(AUTH_COOKIE, token)
        .path("/")
        .http_only(true)
        .secure(config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::seconds(jwt::USER_TOKEN_TTL.num_seconds()))
        .finish();
    if let Some(domain) = &config.cookie_domain {
        cookie.set_domain(domain.clone());
    }
    cookie
}

/// A cookie that overwrites and expires the auth cookie.
pub fn removal_cookie(config: &Config) -> Cookie<'static> {
    let mut cookie = auth_cookie(config, String::new());
    cookie.make_removal();
    cookie
}

/// Extracts the JWT from `Authorization: Bearer <token>` (used by the Next.js server) or,
/// failing that, from the auth cookie (direct browser requests).
fn token_from_request(req: &HttpRequest) -> Option<String> {
    let bearer = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty());
    bearer.or_else(|| req.cookie(AUTH_COOKIE).map(|c| c.value().to_owned()))
}

/// The authenticated user. Add it as a handler argument to require login; requests
/// without a valid JWT are rejected with 401. Extracting it also records the access
/// (status ONLINE, `last_seen` = now).
pub struct AuthUser(pub User);

impl FromRequest for AuthUser {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let state = req.app_data::<web::Data<AppState>>().cloned();
        let token = token_from_request(req);

        Box::pin(async move {
            let state =
                state.ok_or_else(|| ApiError::Internal("AppState not registered".into()))?;
            let token = token.ok_or(ApiError::Unauthorized)?;
            let user_id = state.jwt.verify(&token)?;
            let user = User::touch(&state.db, user_id)
                .await?
                .ok_or(ApiError::Unauthorized)?;
            Ok(AuthUser(user))
        })
    }
}

impl AuthUser {
    /// The user's effective permissions, or 403 unless they include every flag in
    /// `required`.
    pub async fn require(&self, db: &PgPool, required: Permissions) -> Result<Permissions, ApiError> {
        let permissions = Usergroup::permissions_for_user(db, self.0.id).await?;
        if permissions.contains(required) {
            Ok(permissions)
        } else {
            Err(ApiError::Forbidden(required.name()))
        }
    }
}
