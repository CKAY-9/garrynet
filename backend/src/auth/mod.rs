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
    models::{Permissions, Server, User, Usergroup},
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

/// Name of the cookie tying a Steam login to the browser that started it.
pub const LOGIN_STATE_COOKIE: &str = "garrynet_login_state";

/// How long the user has to finish signing in on Steam.
const LOGIN_STATE_TTL_MINUTES: i64 = 10;

/// The login-state cookie, holding the nonce that must come back in Steam's `return_to`. Only
/// sent to the callback; `Lax` still sends it on Steam's top-level redirect back.
pub fn login_state_cookie(config: &Config, nonce: String) -> Cookie<'static> {
    Cookie::build(LOGIN_STATE_COOKIE, nonce)
        .path("/auth/steam/callback")
        .http_only(true)
        .secure(config.cookie_secure)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::minutes(LOGIN_STATE_TTL_MINUTES))
        .finish()
}

/// A cookie that overwrites and expires the login-state cookie.
pub fn login_state_removal_cookie(config: &Config) -> Cookie<'static> {
    let mut cookie = login_state_cookie(config, String::new());
    cookie.make_removal();
    cookie
}

/// Origins (`scheme://host[:port]`) of the frontend and backend: the only sites whose
/// pages may make state-changing requests with the auth cookie. See [`cross_site_write`].
pub fn trusted_origins(config: &Config) -> Vec<String> {
    [&config.frontend_url, &config.backend_url]
        .into_iter()
        .filter_map(|u| url::Url::parse(u).ok())
        .map(|u| u.origin().ascii_serialization())
        .collect()
}

/// Whether `req` is a state-changing request authenticated by the auth cookie that a page
/// on another site sent (CSRF). `SameSite=Lax` keeps the cookie off cross-site POSTs, but
/// not off requests from sibling subdomains, which count as the same site. Requests with a
/// Bearer token don't use the cookie, and ones without an `Origin` header don't come from a
/// browser page, so neither can be forged this way.
pub fn cross_site_write(req: &HttpRequest, trusted: &[String]) -> bool {
    if req.method().is_safe() || bearer_token(req).is_some() || req.cookie(AUTH_COOKIE).is_none() {
        return false;
    }
    match req.headers().get(header::ORIGIN).map(|v| v.to_str()) {
        None => false,
        Some(Ok(origin)) => !trusted.iter().any(|t| t == origin),
        // Not even text (or `null`, from sandboxed frames and the like): not one of ours.
        Some(Err(_)) => true,
    }
}

/// Extracts the JWT from `Authorization: Bearer <token>` (used by the Next.js server) or,
/// failing that, from the auth cookie (direct browser requests).
fn token_from_request(req: &HttpRequest) -> Option<String> {
    bearer_token(req).or_else(|| req.cookie(AUTH_COOKIE).map(|c| c.value().to_owned()))
}

/// The token in `Authorization: Bearer <token>`.
fn bearer_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

/// The value of the `Authorization` header, with or without a `Bearer ` prefix. Used for
/// the access key when a server links.
pub fn authorization(req: &HttpRequest) -> Option<String> {
    bearer_token(req).or_else(|| {
        req.headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
    })
}

/// A linked Garry's Mod server, authenticated by the JWT it got when linking
/// (`Authorization: Bearer <token>`). Requests with a missing, expired or revoked token, or
/// from a server that has since been unlinked or deleted, are rejected with 401.
pub struct AuthServer {
    pub id: i64,
    claims: jwt::ServerClaims,
}

impl FromRequest for AuthServer {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let state = req.app_data::<web::Data<AppState>>().cloned();
        let token = bearer_token(req);

        Box::pin(async move {
            let state =
                state.ok_or_else(|| ApiError::Internal("AppState not registered".into()))?;
            let token = token.ok_or(ApiError::Unauthorized)?;
            let claims = state.jwt.verify_server(&token)?;
            let id = claims.server_id()?;
            match Server::token_version(&state.db, id).await? {
                Some(version) if version == claims.ver => Ok(AuthServer { id, claims }),
                _ => Err(ApiError::Unauthorized),
            }
        })
    }
}

impl AuthServer {
    /// 401 unless the token belongs to server `id` (the one in the request path).
    pub fn require_id(&self, id: i64) -> Result<(), ApiError> {
        if self.id == id { Ok(()) } else { Err(ApiError::Unauthorized) }
    }

    /// A replacement token if the current one is due for rotation.
    pub fn rotated_token(&self, keys: &jwt::JwtKeys) -> Result<Option<String>, ApiError> {
        if self.claims.should_rotate() {
            keys.issue_server(self.id, self.claims.ver).map(Some)
        } else {
            Ok(None)
        }
    }
}

/// The authenticated user. Add it as a handler argument to require login; requests
/// without a valid JWT, or with a revoked one, are rejected with 401. Extracting it also records the access
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
            let claims = state.jwt.verify(&token)?;
            // Also rejects a token revoked since (an old `ver`).
            let user = User::touch(&state.db, claims.user_id()?, claims.ver)
                .await?
                .ok_or(ApiError::Unauthorized)?;
            Ok(AuthUser(user))
        })
    }
}

/// The authenticated user, if any, for routes that also serve visitors. Unlike
/// `Option<AuthUser>`, which turns every failure into "logged out", only a missing, invalid
/// or revoked token gives `None`; other errors (e.g. the database being down) fail the
/// request.
pub struct MaybeUser(pub Option<AuthUser>);

impl FromRequest for MaybeUser {
    type Error = ApiError;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let user = AuthUser::from_request(req, payload);
        Box::pin(async move {
            match user.await {
                Ok(user) => Ok(MaybeUser(Some(user))),
                Err(ApiError::Unauthorized) => Ok(MaybeUser(None)),
                Err(e) => Err(e),
            }
        })
    }
}

impl AuthUser {
    /// The user's effective permissions, or 403 (naming the missing flags) unless they
    /// include every flag in `required`.
    pub async fn require(&self, db: &PgPool, required: Permissions) -> Result<Permissions, ApiError> {
        let permissions = Usergroup::permissions_for_user(db, self.0.id).await?;
        if permissions.contains(required) {
            Ok(permissions)
        } else {
            let missing: Vec<&str> = Permissions::FLAGS
                .iter()
                .filter(|(flag, _)| required.contains(*flag) && !permissions.contains(*flag))
                .map(|(_, name)| *name)
                .collect();
            Err(ApiError::Forbidden(missing.join(", ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{cookie::Cookie, http::Method, test::TestRequest};

    use super::*;

    fn trusted() -> Vec<String> {
        vec!["https://example.com".into(), "https://api.example.com".into()]
    }

    fn post(origin: Option<&str>) -> TestRequest {
        let req = TestRequest::default().method(Method::POST).cookie(Cookie::new(AUTH_COOKIE, "jwt"));
        match origin {
            Some(o) => req.insert_header((header::ORIGIN, o)),
            None => req,
        }
    }

    #[test]
    fn refuses_cookie_writes_from_other_sites() {
        let t = trusted();
        // A sibling subdomain is the same site, so the Lax cookie is sent; the origin isn't ours.
        assert!(cross_site_write(&post(Some("https://forum.example.com")).to_http_request(), &t));
        assert!(cross_site_write(&post(Some("null")).to_http_request(), &t));
        assert!(!cross_site_write(&post(Some("https://example.com")).to_http_request(), &t));
        assert!(!cross_site_write(&post(Some("https://api.example.com")).to_http_request(), &t));
        // No Origin: not a browser page.
        assert!(!cross_site_write(&post(None).to_http_request(), &t));
    }

    #[test]
    fn ignores_reads_bearer_tokens_and_cookieless_requests() {
        let t = trusted();
        let evil = "https://evil.example";
        let get = TestRequest::default()
            .cookie(Cookie::new(AUTH_COOKIE, "jwt"))
            .insert_header((header::ORIGIN, evil));
        assert!(!cross_site_write(&get.to_http_request(), &t));
        let bearer = post(Some(evil)).insert_header((header::AUTHORIZATION, "Bearer jwt"));
        assert!(!cross_site_write(&bearer.to_http_request(), &t));
        let no_cookie = TestRequest::default().method(Method::POST).insert_header((header::ORIGIN, evil));
        assert!(!cross_site_write(&no_cookie.to_http_request(), &t));
    }
}
