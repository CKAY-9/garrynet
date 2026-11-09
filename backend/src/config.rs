use std::env;

/// Runtime configuration, loaded from environment variables (and `.env` in development).
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    /// Public URL of the backend as seen by the browser, without a trailing slash.
    pub backend_url: String,
    /// Public URL of the frontend, without a trailing slash.
    pub frontend_url: String,
    pub jwt_secret: String,
    pub steam_api_key: String,
    pub cookie_domain: Option<String>,
    pub cookie_secure: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let backend_url = required("BACKEND_URL")?.trim_end_matches('/').to_owned();
        let frontend_url = required("FRONTEND_URL")?.trim_end_matches('/').to_owned();

        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 bytes long".into());
        }

        let cookie_secure = match optional("COOKIE_SECURE") {
            Some(v) => v
                .parse()
                .map_err(|_| "COOKIE_SECURE must be `true` or `false`".to_owned())?,
            None => backend_url.starts_with("https://"),
        };

        Ok(Self {
            bind_addr: optional("BIND_ADDR").unwrap_or_else(|| "127.0.0.1:8080".into()),
            database_url: required("DATABASE_URL")?,
            backend_url,
            frontend_url,
            jwt_secret,
            steam_api_key: required("STEAM_API_KEY")?,
            cookie_domain: optional("COOKIE_DOMAIN"),
            cookie_secure,
        })
    }

    /// URL Steam redirects back to after the user signs in.
    pub fn steam_return_to(&self) -> String {
        format!("{}/auth/steam/callback", self.backend_url)
    }
}

fn required(key: &str) -> Result<String, String> {
    optional(key).ok_or_else(|| format!("missing required environment variable {key}"))
}

fn optional(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}
