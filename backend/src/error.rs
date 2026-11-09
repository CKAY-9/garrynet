use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("authentication required")]
    Unauthorized,

    #[error("missing permission: {0}")]
    Forbidden(&'static str),

    #[error("{0} not found")]
    NotFound(&'static str),

    #[error("steam login could not be verified: {0}")]
    SteamVerification(String),

    #[error("steam is unavailable: {0}")]
    SteamUnavailable(String),

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
            Self::SteamVerification(_) => StatusCode::UNAUTHORIZED,
            Self::SteamUnavailable(_) => StatusCode::BAD_GATEWAY,
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
                _ => "internal server error".to_owned(),
            }
        } else {
            log::debug!("{self}");
            self.to_string()
        };
        HttpResponse::build(status).json(json!({ "error": message }))
    }
}
