use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authentication required")]
    Unauthorized,

    /// What the user lacks: the names of the missing permissions, or a role such as
    /// "message author".
    #[error("missing permission: {0}")]
    Forbidden(String),

    #[error("{0} not found")]
    NotFound(&'static str),

    /// The request body failed validation; the message says which field and why.
    #[error("{0}")]
    BadRequest(String),

    #[error("{0} already exists")]
    Conflict(&'static str),

    /// The forum category or thread is locked against changes.
    #[error("this {0} is locked")]
    Locked(&'static str),

    /// Too many requests; the number is how many seconds to wait.
    #[error("too many requests, try again in {0}s")]
    RateLimited(u64),

    #[error("steam login could not be verified: {0}")]
    SteamVerification(String),

    #[error("steam is unavailable: {0}")]
    SteamUnavailable(String),

    /// Stripe refused a request we made (e.g. an amount below its minimum). The message is
    /// Stripe's and is safe to show.
    #[error("payment provider rejected the request: {0}")]
    StripeRejected(String),

    /// Stripe couldn't be reached or answered unexpectedly.
    #[error("stripe is unavailable: {0}")]
    StripeUnavailable(String),

    /// A feature that needs configuration this deployment doesn't have (e.g. Stripe keys).
    #[error("{0} is not configured on this GarryNet")]
    NotConfigured(&'static str),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Locked(_) => StatusCode::LOCKED,
            Self::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::SteamVerification(_) => StatusCode::UNAUTHORIZED,
            Self::StripeRejected(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::SteamUnavailable(_) | Self::StripeUnavailable(_) => StatusCode::BAD_GATEWAY,
            Self::NotConfigured(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let message = if status.is_server_error() {
            log::error!("{self}");
            // Don't leak internals (SQL errors etc.) to clients.
            match self {
                Self::SteamUnavailable(_) => "steam is unavailable, try again later".to_owned(),
                Self::StripeUnavailable(_) => "payments are unavailable, try again later".to_owned(),
                Self::NotConfigured(_) => self.to_string(),
                _ => "internal server error".to_owned(),
            }
        } else {
            log::debug!("{self}");
            self.to_string()
        };
        let mut response = HttpResponse::build(status);
        if let Self::RateLimited(secs) = self {
            response.insert_header((actix_web::http::header::RETRY_AFTER, secs.to_string()));
        }
        response.json(json!({ "error": message }))
    }
}
