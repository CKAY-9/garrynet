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
    /// `None` until Stripe is configured; the store can be browsed and managed but nothing
    /// can be bought.
    pub stripe: Option<StripeConfig>,
}

/// Stripe settings (docs/STORE.md). All or nothing: setting some of the variables but not
/// the others is a configuration error.
#[derive(Clone, Debug)]
pub struct StripeConfig {
    pub secret_key: String,
    /// Signing secret of the webhook endpoint pointed at `POST /stripe/webhook`.
    pub webhook_secret: String,
    /// Whether Checkout Sessions ask Stripe Tax to add tax (a deployment decision).
    pub tax: bool,
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
            stripe: stripe_from_env()?,
        })
    }

    /// URL Steam redirects back to after the user signs in.
    pub fn steam_return_to(&self) -> String {
        format!("{}/auth/steam/callback", self.backend_url)
    }
}

fn stripe_from_env() -> Result<Option<StripeConfig>, String> {
    let secret_key = optional("STRIPE_SECRET_KEY");
    let webhook_secret = optional("STRIPE_WEBHOOK_SECRET");
    let tax = match optional("STRIPE_TAX") {
        Some(v) => v
            .parse()
            .map_err(|_| "STRIPE_TAX must be `true` or `false`".to_owned())?,
        None => false,
    };
    match (secret_key, webhook_secret) {
        (None, None) => Ok(None),
        (Some(secret_key), Some(webhook_secret)) => {
            if !secret_key.starts_with("sk_") && !secret_key.starts_with("rk_") {
                return Err("STRIPE_SECRET_KEY must be a Stripe secret key (sk_... or rk_...)".into());
            }
            Ok(Some(StripeConfig { secret_key, webhook_secret, tax }))
        }
        _ => Err("STRIPE_SECRET_KEY and STRIPE_WEBHOOK_SECRET must be set together".into()),
    }
}

fn required(key: &str) -> Result<String, String> {
    optional(key).ok_or_else(|| format!("missing required environment variable {key}"))
}

fn optional(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}
