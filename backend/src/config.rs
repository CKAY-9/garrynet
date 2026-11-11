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
    pub storage: StorageConfig,
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
    /// With `tax`, whether item prices include tax or have it added on top.
    pub tax_behavior: TaxBehavior,
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
            storage: storage_from_env()?,
        })
    }

    /// URL Steam redirects back to after the user signs in, carrying the login's `state`
    /// nonce (hex, so it needs no encoding). See `routes::auth`.
    pub fn steam_return_to(&self, state: &str) -> String {
        format!("{}/auth/steam/callback?state={state}", self.backend_url)
    }
}

/// Whether item prices include tax (`inclusive`) or Stripe Tax adds it on top (`exclusive`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaxBehavior {
    Exclusive,
    Inclusive,
}

impl TaxBehavior {
    /// As Stripe's `tax_behavior` takes it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exclusive => "exclusive",
            Self::Inclusive => "inclusive",
        }
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
    let tax_behavior = match optional("STRIPE_TAX_BEHAVIOR").as_deref() {
        None | Some("exclusive") => TaxBehavior::Exclusive,
        Some("inclusive") => TaxBehavior::Inclusive,
        Some(_) => return Err("STRIPE_TAX_BEHAVIOR must be `exclusive` or `inclusive`".into()),
    };
    match (secret_key, webhook_secret) {
        (None, None) => Ok(None),
        (Some(secret_key), Some(webhook_secret)) => {
            if !secret_key.starts_with("sk_") && !secret_key.starts_with("rk_") {
                return Err("STRIPE_SECRET_KEY must be a Stripe secret key (sk_... or rk_...)".into());
            }
            Ok(Some(StripeConfig { secret_key, webhook_secret, tax, tax_behavior }))
        }
        _ => Err("STRIPE_SECRET_KEY and STRIPE_WEBHOOK_SECRET must be set together".into()),
    }
}

/// Which storage provider new uploads go to (docs/STORAGE.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageProvider {
    Local,
    S3,
}

/// File storage settings (docs/STORAGE.md).
#[derive(Clone, Debug)]
pub struct StorageConfig {
    /// Where new uploads go. Files keep the provider they were uploaded to.
    pub provider: StorageProvider,
    /// Root directory of the local provider.
    pub local_path: String,
    /// `None` unless `S3_BUCKET` is set.
    pub s3: Option<S3Config>,
    /// Largest upload accepted, in bytes.
    pub max_file_size: u64,
    /// Minimum time between two uploads by the same user.
    pub rate_limit_secs: u64,
}

/// An S3-compatible bucket: AWS S3, Cloudflare R2, MinIO, ...
#[derive(Clone, Debug)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    /// Custom endpoint, e.g. `https://<account>.r2.cloudflarestorage.com`. `None` for AWS.
    pub endpoint: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    /// Public base URL of the bucket (custom domain or CDN), without a trailing slash. When
    /// set, files are read from `{public_url}/{key}` instead of short-lived presigned URLs.
    pub public_url: Option<String>,
}

fn storage_from_env() -> Result<StorageConfig, String> {
    let provider = match optional("STORAGE_PROVIDER").as_deref() {
        None | Some("local") => StorageProvider::Local,
        Some("s3") => StorageProvider::S3,
        Some(other) => return Err(format!("STORAGE_PROVIDER must be `local` or `s3`, got {other:?}")),
    };
    let s3 = match optional("S3_BUCKET") {
        None => None,
        Some(bucket) => Some(S3Config {
            bucket,
            // R2 ignores the region but the signature needs one; it accepts `auto`.
            region: optional("S3_REGION").unwrap_or_else(|| "us-east-1".into()),
            endpoint: optional("S3_ENDPOINT").map(|v| v.trim_end_matches('/').to_owned()),
            access_key_id: required("S3_ACCESS_KEY_ID")?,
            secret_access_key: required("S3_SECRET_ACCESS_KEY")?,
            public_url: optional("S3_PUBLIC_URL").map(|v| v.trim_end_matches('/').to_owned()),
        }),
    };
    if provider == StorageProvider::S3 && s3.is_none() {
        return Err("STORAGE_PROVIDER=s3 needs S3_BUCKET, S3_ACCESS_KEY_ID and S3_SECRET_ACCESS_KEY".into());
    }
    let number = |key: &str, default: u64| -> Result<u64, String> {
        optional(key).map_or(Ok(default), |v| {
            v.parse().map_err(|_| format!("{key} must be a whole number, got {v:?}"))
        })
    };
    let max_file_size = number("STORAGE_MAX_FILE_SIZE", 2 * 1024 * 1024)?;
    if max_file_size == 0 {
        return Err("STORAGE_MAX_FILE_SIZE must be greater than 0".into());
    }
    Ok(StorageConfig {
        provider,
        local_path: optional("STORAGE_LOCAL_PATH").unwrap_or_else(|| "./storage".into()),
        s3,
        max_file_size,
        rate_limit_secs: number("STORAGE_RATE_LIMIT", 3)?,
    })
}

fn required(key: &str) -> Result<String, String> {
    optional(key).ok_or_else(|| format!("missing required environment variable {key}"))
}

fn optional(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}
