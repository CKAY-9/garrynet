use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, models::User};

/// Standard user tokens are valid for 24 hours.
pub const USER_TOKEN_TTL: Duration = Duration::hours(24);

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// GarryNet user id.
    pub sub: String,
    pub steam_id: String,
    pub iat: i64,
    pub exp: i64,
}

#[derive(Clone)]
pub struct JwtKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl JwtKeys {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn issue(&self, user: &User) -> Result<String, ApiError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user.id.to_string(),
            steam_id: user.steam_id.clone(),
            iat: now.timestamp(),
            exp: (now + USER_TOKEN_TTL).timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| ApiError::Internal(format!("failed to sign JWT: {e}")))
    }

    /// Validates the signature and expiry, returning the user id the token was issued for.
    pub fn verify(&self, token: &str) -> Result<i64, ApiError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0;
        let data = decode::<Claims>(token, &self.decoding, &validation)
            .map_err(|_| ApiError::Unauthorized)?;
        data.claims.sub.parse().map_err(|_| ApiError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserStatus;

    fn user() -> User {
        User {
            id: 42,
            username: "garry".into(),
            email: None,
            avatar_url: String::new(),
            steam_id: "76561197960287930".into(),
            bio: String::new(),
            reputation: 0,
            status: UserStatus::Online,
            joined: Utc::now(),
            last_seen: Utc::now(),
        }
    }

    const SECRET: &str = "test-secret-test-secret-test-secret";

    #[test]
    fn round_trip() {
        let keys = JwtKeys::new(SECRET);
        let token = keys.issue(&user()).unwrap();
        assert_eq!(keys.verify(&token).unwrap(), 42);
    }

    #[test]
    fn rejects_wrong_secret() {
        let token = JwtKeys::new(SECRET).issue(&user()).unwrap();
        let other = JwtKeys::new("another-secret-another-secret-another");
        assert!(other.verify(&token).is_err());
    }

    #[test]
    fn rejects_expired() {
        let keys = JwtKeys::new(SECRET);
        let past = Utc::now() - Duration::hours(25);
        let claims = Claims {
            sub: "42".into(),
            steam_id: "76561197960287930".into(),
            iat: past.timestamp(),
            exp: (past + USER_TOKEN_TTL).timestamp(),
        };
        let token = encode(&Header::default(), &claims, &keys.encoding).unwrap();
        assert!(keys.verify(&token).is_err());
    }
}
