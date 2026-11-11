use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, models::User};

/// Standard user tokens are valid for 24 hours.
pub const USER_TOKEN_TTL: Duration = Duration::hours(24);

/// How long a server token is valid. A server that stays offline for longer has to be
/// linked again with a new access key.
pub const SERVER_TOKEN_TTL: Duration = Duration::days(30);

/// Server tokens are rolling: once a token is this old, the next authenticated response
/// carries a replacement.
pub const SERVER_TOKEN_ROTATE_AFTER: Duration = Duration::hours(1);

/// `aud` of server tokens. User tokens have none, so neither kind passes for the other.
const SERVER_AUDIENCE: &str = "garrynet-server";

/// Claims of a token issued to a Garry's Mod server running the addon.
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerClaims {
    /// GarryNet server id.
    pub sub: String,
    pub aud: String,
    /// `servers.token_version` when the token was issued; the token is only accepted while
    /// it still matches.
    pub ver: i64,
    pub iat: i64,
    pub exp: i64,
}

impl ServerClaims {
    pub fn server_id(&self) -> Result<i64, ApiError> {
        self.sub.parse().map_err(|_| ApiError::Unauthorized)
    }

    /// Whether the token is old enough to be replaced.
    pub fn should_rotate(&self) -> bool {
        Utc::now().timestamp() - self.iat >= SERVER_TOKEN_ROTATE_AFTER.num_seconds()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// GarryNet user id.
    pub sub: String,
    pub steam_id: String,
    /// `users.token_version` when the token was issued; the token is only accepted while it
    /// still matches. Missing in tokens issued before versions existed, which read as 0.
    #[serde(default)]
    pub ver: i64,
    pub iat: i64,
    pub exp: i64,
}

impl Claims {
    pub fn user_id(&self) -> Result<i64, ApiError> {
        self.sub.parse().map_err(|_| ApiError::Unauthorized)
    }
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
            ver: user.token_version,
            iat: now.timestamp(),
            exp: (now + USER_TOKEN_TTL).timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| ApiError::Internal(format!("failed to sign JWT: {e}")))
    }

    /// Validates the signature and expiry, returning the token's claims. The caller still
    /// has to check `ver` against the database.
    pub fn verify(&self, token: &str) -> Result<Claims, ApiError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0;
        decode::<Claims>(token, &self.decoding, &validation)
            .map(|data| data.claims)
            .map_err(|_| ApiError::Unauthorized)
    }

    /// Issues a token for server `server_id` at `servers.token_version` = `version`.
    pub fn issue_server(&self, server_id: i64, version: i64) -> Result<String, ApiError> {
        let now = Utc::now();
        let claims = ServerClaims {
            sub: server_id.to_string(),
            aud: SERVER_AUDIENCE.to_owned(),
            ver: version,
            iat: now.timestamp(),
            exp: (now + SERVER_TOKEN_TTL).timestamp(),
        };
        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| ApiError::Internal(format!("failed to sign server JWT: {e}")))
    }

    /// Validates a server token's signature, audience and expiry. The caller still has to
    /// check `ver` against the database.
    pub fn verify_server(&self, token: &str) -> Result<ServerClaims, ApiError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0;
        validation.set_audience(&[SERVER_AUDIENCE]);
        decode::<ServerClaims>(token, &self.decoding, &validation)
            .map(|data| data.claims)
            .map_err(|_| ApiError::Unauthorized)
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
            token_version: 3,
        }
    }

    const SECRET: &str = "test-secret-test-secret-test-secret";

    #[test]
    fn round_trip() {
        let keys = JwtKeys::new(SECRET);
        let token = keys.issue(&user()).unwrap();
        let claims = keys.verify(&token).unwrap();
        assert_eq!(claims.user_id().unwrap(), 42);
        assert_eq!(claims.ver, 3);
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
            ver: 0,
            iat: past.timestamp(),
            exp: (past + USER_TOKEN_TTL).timestamp(),
        };
        let token = encode(&Header::default(), &claims, &keys.encoding).unwrap();
        assert!(keys.verify(&token).is_err());
    }

    #[test]
    fn tokens_without_a_version_read_as_zero() {
        #[derive(Serialize)]
        struct Old {
            sub: String,
            steam_id: String,
            iat: i64,
            exp: i64,
        }
        let keys = JwtKeys::new(SECRET);
        let now = Utc::now();
        let old = Old {
            sub: "42".into(),
            steam_id: "76561197960287930".into(),
            iat: now.timestamp(),
            exp: (now + USER_TOKEN_TTL).timestamp(),
        };
        let token = encode(&Header::default(), &old, &keys.encoding).unwrap();
        assert_eq!(keys.verify(&token).unwrap().ver, 0);
    }

    #[test]
    fn server_round_trip() {
        let keys = JwtKeys::new(SECRET);
        let claims = keys.verify_server(&keys.issue_server(7, 3).unwrap()).unwrap();
        assert_eq!(claims.server_id().unwrap(), 7);
        assert_eq!(claims.ver, 3);
        assert!(!claims.should_rotate());
    }

    #[test]
    fn user_and_server_tokens_are_not_interchangeable() {
        let keys = JwtKeys::new(SECRET);
        let user_token = keys.issue(&user()).unwrap();
        let server_token = keys.issue_server(42, 1).unwrap();
        assert!(keys.verify_server(&user_token).is_err());
        assert!(keys.verify(&server_token).is_err());
    }

    #[test]
    fn rotates_old_server_tokens() {
        let issued = Utc::now() - SERVER_TOKEN_ROTATE_AFTER - Duration::seconds(1);
        let claims = ServerClaims {
            sub: "1".into(),
            aud: SERVER_AUDIENCE.into(),
            ver: 1,
            iat: issued.timestamp(),
            exp: (issued + SERVER_TOKEN_TTL).timestamp(),
        };
        assert!(claims.should_rotate());
    }
}
