use std::env;

use sqlx::postgres::PgConnectOptions;

/// Runtime configuration, loaded from environment variables (and `.env` in development).
#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    /// `DATABASE_URL`, with the password from `DATABASE_PASSWORD_FILE` if that's set.
    pub database: PgConnectOptions,
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
    /// Demo mode (docs/DEMO.md): Stripe must use test keys, and what non-admins create is
    /// cleared every day.
    pub demo: bool,
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

        let demo = match optional("DEMO_MODE") {
            Some(v) => v
                .parse()
                .map_err(|_| "DEMO_MODE must be `true` or `false`".to_owned())?,
            None => false,
        };

        require_https("BACKEND_URL", &backend_url, Reach::Public)?;
        require_https("FRONTEND_URL", &frontend_url, Reach::Public)?;

        let cookie_secure = match optional("COOKIE_SECURE") {
            Some(v) => v
                .parse()
                .map_err(|_| "COOKIE_SECURE must be `true` or `false`".to_owned())?,
            None => backend_url.starts_with("https://"),
        };
        if !cookie_secure && !is_local(&backend_url, Reach::Public) {
            insecure("COOKIE_SECURE=false sends the login cookie over plain HTTP")?;
        }

        Ok(Self {
            bind_addr: optional("BIND_ADDR").unwrap_or_else(|| "127.0.0.1:8080".into()),
            database: database_from_env()?,
            backend_url,
            frontend_url,
            jwt_secret,
            steam_api_key: required("STEAM_API_KEY")?,
            cookie_domain: optional("COOKIE_DOMAIN"),
            cookie_secure,
            stripe: stripe_from_env(demo)?,
            storage: storage_from_env()?,
            demo,
        })
    }

    /// URL Steam redirects back to after the user signs in, carrying the login's `state`
    /// nonce (hex, so it needs no encoding). See `routes::auth`.
    /// Whether the backend is served over HTTPS (its public URL is `https://`), so browsers
    /// can be told to always use it (HSTS).
    pub fn https(&self) -> bool {
        self.backend_url.starts_with("https://")
    }

    pub fn steam_return_to(&self, state: &str) -> String {
        format!("{}/auth/steam/callback?state={state}", self.backend_url)
    }

    /// Names this deployment to Stripe (docs/STORE.md): the backend's host, and port if it
    /// has one. Deployments sharing a Stripe account (staging and production, say) get each
    /// other's webhooks, and their purchase ids overlap.
    pub fn deployment(&self) -> String {
        deployment(&self.backend_url)
    }
}

fn deployment(backend_url: &str) -> String {
    match url::Url::parse(backend_url) {
        Ok(url) => match (url.host_str(), url.port()) {
            (Some(host), Some(port)) => format!("{host}:{port}"),
            (Some(host), None) => host.to_owned(),
            (None, _) => backend_url.to_owned(),
        },
        Err(_) => backend_url.to_owned(),
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

fn stripe_from_env(demo: bool) -> Result<Option<StripeConfig>, String> {
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
            if demo && !is_test_key(&secret_key) {
                return Err("DEMO_MODE only takes Stripe test keys: STRIPE_SECRET_KEY must start with sk_test_ or rk_test_".into());
            }
            Ok(Some(StripeConfig { secret_key, webhook_secret, tax, tax_behavior }))
        }
        _ => Err("STRIPE_SECRET_KEY and STRIPE_WEBHOOK_SECRET must be set together".into()),
    }
}

/// Whether a Stripe secret key is a test-mode key, which can't move real money.
fn is_test_key(key: &str) -> bool {
    key.starts_with("sk_test_") || key.starts_with("rk_test_")
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
    /// Most bytes a user may hold in files not attached to anything yet (pending uploads
    /// included), so uploads can't be used as free storage until the sweeper clears them.
    pub user_quota: u64,
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
            endpoint: optional("S3_ENDPOINT")
                .map(|v| v.trim_end_matches('/').to_owned())
                .map(|v| require_https("S3_ENDPOINT", &v, Reach::Internal).map(|()| v))
                .transpose()?,
            access_key_id: required("S3_ACCESS_KEY_ID")?,
            secret_access_key: required("S3_SECRET_ACCESS_KEY")?,
            public_url: optional("S3_PUBLIC_URL")
                .map(|v| v.trim_end_matches('/').to_owned())
                .map(|v| require_https("S3_PUBLIC_URL", &v, Reach::Public).map(|()| v))
                .transpose()?,
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
    // Enough for twice the attachments one message can have (10) at the largest size.
    let user_quota = number("STORAGE_USER_QUOTA", max_file_size.saturating_mul(20))?;
    if user_quota < max_file_size {
        return Err("STORAGE_USER_QUOTA must be at least STORAGE_MAX_FILE_SIZE".into());
    }
    Ok(StorageConfig {
        provider,
        local_path: optional("STORAGE_LOCAL_PATH").unwrap_or_else(|| "./storage".into()),
        s3,
        max_file_size,
        rate_limit_secs: number("STORAGE_RATE_LIMIT", 3)?,
        user_quota,
    })
}

/// How to reach Postgres: `DATABASE_URL`, with its password replaced by the contents of
/// `DATABASE_PASSWORD_FILE` when that's set (Docker Compose generates the password into a file
/// shared with the database; see docker-compose.yml). Trailing newlines in the file are ignored.
pub fn database_from_env() -> Result<PgConnectOptions, String> {
    let url = required("DATABASE_URL")?;
    let options: PgConnectOptions = url.parse().map_err(|e| format!("invalid DATABASE_URL: {e}"))?;
    let Some(path) = optional("DATABASE_PASSWORD_FILE") else {
        return Ok(options);
    };
    let password = std::fs::read_to_string(&path)
        .map_err(|e| format!("can't read DATABASE_PASSWORD_FILE {path:?}: {e}"))?;
    let password = password.trim_end_matches(['\r', '\n']);
    if password.is_empty() {
        return Err(format!("DATABASE_PASSWORD_FILE {path:?} is empty"));
    }
    Ok(options.password(password))
}

/// Development builds (`cargo run`, without `--release`) accept plain HTTP anywhere, with a
/// warning; release builds refuse it outside this machine.
const DEVELOPMENT: bool = cfg!(debug_assertions);

/// Who has to reach a URL, which decides where plain HTTP is acceptable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reach {
    /// Browsers or game servers: HTTP only on this machine (`localhost`, `*.localhost` or a
    /// loopback address).
    Public,
    /// Only this backend: also a single-label name (`minio`, `garage`), as services on a
    /// private network such as Docker Compose's are named.
    Internal,
}

/// Whether `url`'s host never leaves this machine (or, for [`Reach::Internal`], the private
/// network it's named on). Unparsable URLs aren't local.
fn is_local(url: &str, reach: Reach) -> bool {
    let Ok(url) = url::Url::parse(url) else { return false };
    match url.host() {
        Some(url::Host::Domain(name)) => {
            let name = name.trim_end_matches('.').to_ascii_lowercase();
            name == "localhost"
                || name.ends_with(".localhost")
                || (reach == Reach::Internal && !name.is_empty() && !name.contains('.'))
        }
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}

/// HTTPS is required outside this machine (see [`Reach`]): login cookies, tokens, server
/// keys and uploads would otherwise cross the network in the clear.
fn require_https(key: &str, url: &str, reach: Reach) -> Result<(), String> {
    if url.to_ascii_lowercase().starts_with("https://") || is_local(url, reach) {
        return Ok(());
    }
    insecure(&format!("{key} must be an https:// URL outside localhost, got {url:?}"))
}

/// A configuration error, except in development builds, where it's only a warning.
fn insecure(problem: &str) -> Result<(), String> {
    if DEVELOPMENT {
        log::warn!("{problem}; allowed only because this is a development build");
        Ok(())
    } else {
        Err(problem.to_owned())
    }
}

fn required(key: &str) -> Result<String, String> {
    optional(key).ok_or_else(|| format!("missing required environment variable {key}"))
}

fn optional(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keys() {
        assert!(is_test_key("sk_test_abc"));
        assert!(is_test_key("rk_test_abc"));
        assert!(!is_test_key("sk_live_abc"));
        assert!(!is_test_key("rk_live_abc"));
        assert!(!is_test_key("sk_abc"));
    }

    #[test]
    fn local_hosts() {
        for url in ["http://localhost:8080", "http://LOCALHOST", "http://api.localhost", "http://127.0.0.1:3000", "http://127.8.0.1", "http://[::1]:8080"] {
            assert!(is_local(url, Reach::Public), "{url}");
        }
        for url in ["http://example.com", "http://192.168.1.10:8080", "http://10.0.0.1", "http://localhost.example.com", "http://minio:9000", "not a url"] {
            assert!(!is_local(url, Reach::Public), "{url}");
        }
        assert!(is_local("http://minio:9000", Reach::Internal));
        assert!(is_local("http://garage:3900", Reach::Internal));
        assert!(!is_local("http://s3.example.com", Reach::Internal));
        assert!(!is_local("http://192.168.1.10:9000", Reach::Internal));
    }

    #[test]
    fn https_required() {
        assert!(require_https("X", "https://api.example.com", Reach::Public).is_ok());
        assert!(require_https("X", "HTTPS://api.example.com", Reach::Public).is_ok());
        assert!(require_https("X", "http://localhost:8080", Reach::Public).is_ok());
        assert!(require_https("X", "http://minio:9000", Reach::Internal).is_ok());
        // Development (test) builds only warn.
        assert_eq!(require_https("X", "http://api.example.com", Reach::Public).is_ok(), DEVELOPMENT);
    }

    #[test]
    fn deployments() {
        assert_eq!(deployment("https://api.example.com"), "api.example.com");
        assert_eq!(deployment("https://API.example.com/"), "api.example.com");
        assert_eq!(deployment("http://localhost:8080"), "localhost:8080");
        assert_eq!(deployment("not a url"), "not a url");
    }
}
