mod auth;
mod config;
mod demo;
mod error;
mod hex;
mod models;
mod notify;
mod routes;
mod shell;
mod state;
mod steam_profiles;
mod storage;
mod stripe;

use std::{sync::Arc, time::Duration};

use actix_web::{
    App, HttpServer,
    dev::{Service, ServiceResponse},
    http::header,
    middleware::{Condition, DefaultHeaders, Logger},
    web,
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use crate::{
    auth::jwt::JwtKeys,
    config::Config,
    error::ApiError,
    models::{
        Community, File, Usergroup, legal::LegalDocument, notification::Notification,
        store::Receipt, support::SupportTicket,
    },
    state::AppState,
    storage::Storage,
    stripe::Stripe,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::Builder::new()
        // Postgres reports "relation _sqlx_migrations already exists" on every start.
        // RUST_LOG can still override this.
        .filter_module("sqlx::postgres::notice", log::LevelFilter::Warn)
        .parse_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    // reqwest is built without a bundled crypto provider; use ring, as sqlx does.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("failed to install rustls crypto provider");

    // `backend <command...>` runs a single shell command (e.g. `usergroup add 1 admin`)
    // and exits instead of starting the server.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        return run_command(&args).await;
    }

    let config = load_config();
    let state = build_state(config.clone()).await?;
    if config.demo {
        log::warn!(
            "DEMO_MODE is on: everything users without Admin Access create is deleted at midnight UTC (next reset {})",
            demo::next_reset(chrono::Utc::now())
        );
    }

    log::info!("GravNet backend listening on http://{}", config.bind_addr);
    shell::spawn(state.clone());
    if config.demo {
        demo::spawn_daily_reset(state.clone());
    }
    spawn_ticket_auto_close(state.db.clone());
    spawn_purchase_sweeper(state.clone());
    spawn_upload_sweeper(state.db.clone(), state.storage.clone());
    spawn_notification_sweeper(state.db.clone());

    let trusted_origins = Arc::new(auth::trusted_origins(&config));
    let https = config.https();
    HttpServer::new(move || {
        let trusted_origins = trusted_origins.clone();
        App::new()
            .app_data(web::Data::new(state.clone()))
            // Refuse cookie-authenticated writes from other sites' pages (CSRF).
            .wrap_fn(move |req, srv| {
                let forged = auth::cross_site_write(req.request(), &trusted_origins);
                let res = (!forged).then(|| srv.call(req));
                async move {
                    match res {
                        Some(res) => res.await.map(ServiceResponse::map_into_boxed_body),
                        None => Err(ApiError::Forbidden(
                            "a request from this site (the login cookie can't be used from other sites)".into(),
                        )
                        .into()),
                    }
                }
            })
            .wrap(Logger::new("%r %s %Dms"))
            // Served over HTTPS: browsers that have seen it never use plain HTTP for it again,
            // for a year (HSTS). Not for subdomains, which may be other people's services.
            .wrap(Condition::new(
                https,
                DefaultHeaders::new().add((header::STRICT_TRANSPORT_SECURITY, HSTS)),
            ))
            .configure(routes::configure)
    })
    .bind(&config.bind_addr)?
    .run()
    .await
}

/// `Strict-Transport-Security` when the backend is served over HTTPS: a year, this host only.
const HSTS: &str = "max-age=31536000";

/// The configuration from the environment; exits on a configuration error.
fn load_config() -> Config {
    Config::from_env().unwrap_or_else(|e| {
        eprintln!("configuration error: {e}");
        std::process::exit(1);
    })
}

/// Connects to the database and sets up the clients the server (or a command) needs.
async fn build_state(config: Config) -> std::io::Result<AppState> {
    let db = connect_db(config.database.clone()).await?;

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(std::io::Error::other)?;

    let stripe = config
        .stripe
        .clone()
        .map(|stripe| Stripe::new(http.clone(), stripe, config.deployment()));
    if stripe.is_none() {
        log::warn!("STRIPE_SECRET_KEY and STRIPE_WEBHOOK_SECRET are not set: the store can't take payments");
    }

    let storage = Storage::new(&config.storage, &config.backend_url, &config.jwt_secret)
        .unwrap_or_else(|e| {
            eprintln!("configuration error: {e}");
            std::process::exit(1);
        });
    log::info!("storing new uploads with the {} provider", storage.default_backend());

    Ok(AppState {
        db,
        jwt: JwtKeys::new(&config.jwt_secret),
        config: Arc::new(config),
        http,
        stripe,
        storage,
        steam_profiles: Arc::default(),
    })
}

/// Connects to Postgres, applies migrations and creates the default usergroups, the
/// skeleton community and the default legal documents.
async fn connect_db(options: PgConnectOptions) -> std::io::Result<PgPool> {
    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await
        .map_err(std::io::Error::other)?;
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .map_err(std::io::Error::other)?;
    Usergroup::ensure_defaults(&db)
        .await
        .map_err(std::io::Error::other)?;
    Community::ensure_exists(&db)
        .await
        .map_err(std::io::Error::other)?;
    LegalDocument::ensure_seeded(&db)
        .await
        .map_err(std::io::Error::other)?;
    Ok(db)
}

/// Closes support tickets that have waited on their user for too long (see
/// `SupportTicket::auto_close`), once now and then every hour.
fn spawn_ticket_auto_close(db: PgPool) {
    actix_web::rt::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            tick.tick().await;
            match SupportTicket::auto_close(&db).await {
                Ok(closed) if closed.is_empty() => {}
                Ok(closed) => {
                    log::info!("automatically closed {} inactive support ticket(s)", closed.len());
                    for ticket in &closed {
                        notify::ticket_auto_closed(&db, ticket).await;
                    }
                }
                Err(e) => log::error!("failed to auto-close support tickets: {e}"),
            }
        }
    });
}

/// Applies notification retention (see `Notification::sweep`), once now and then every hour.
fn spawn_notification_sweeper(db: PgPool) {
    actix_web::rt::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60 * 60));
        loop {
            tick.tick().await;
            match Notification::sweep(&db).await {
                Ok(0) => {}
                Ok(n) => log::info!("deleted {n} old notification(s)"),
                Err(e) => log::error!("failed to sweep notifications: {e}"),
            }
        }
    });
}

/// Checkout Sessions expire after 35 minutes; a pending purchase this old has missed its
/// webhook (or is a delayed payment still clearing).
const STALE_PURCHASE_MINUTES: i64 = 90;

/// Refunds and disputes reported for a payment that still isn't recorded after this long are
/// forgotten: the payment isn't GravNet's. Stripe retries a failing webhook for 3 days.
const EARLY_SETTLEMENT_DAYS: i32 = 30;

/// Settles store purchases whose Checkout Session should long have been settled by a
/// webhook (see `routes::store::sweep_stale_purchases`), and forgets refunds and disputes of
/// payments that never turned up (see `Receipt::settle_payment`). Every 10 minutes.
fn spawn_purchase_sweeper(state: AppState) {
    actix_web::rt::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(10 * 60));
        loop {
            tick.tick().await;
            match routes::store::sweep_stale_purchases(&state, STALE_PURCHASE_MINUTES).await {
                Ok((0, 0)) => {}
                Ok((cancelled, paid)) => {
                    log::info!("stale store purchases: {cancelled} cancelled, {paid} recorded as paid")
                }
                Err(e) => log::error!("failed to sweep stale store purchases: {e}"),
            }
            match Receipt::sweep_early_settlements(&state.db, EARLY_SETTLEMENT_DAYS).await {
                Ok(0) => {}
                Ok(n) => log::info!("forgot {n} refund(s) or dispute(s) of payments that were never recorded"),
                Err(e) => log::error!("failed to sweep early store settlements: {e}"),
            }
        }
    });
}

/// Upload targets expire after 15 minutes; a file still pending this long was abandoned.
const STALE_UPLOAD_MINUTES: i64 = 60;

/// A user file still unattached this long after its upload was never used. Generous, since
/// a post can take a while to write after its files are uploaded.
const UNATTACHED_FILE_HOURS: i64 = 24;

/// Deletes abandoned uploads and unused files (rows and their bytes), every 10 minutes.
fn spawn_upload_sweeper(db: PgPool, storage: Storage) {
    actix_web::rt::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(10 * 60));
        loop {
            tick.tick().await;
            sweep_files(&db, &storage, Sweep::Abandoned).await;
            sweep_files(&db, &storage, Sweep::Unattached).await;
            sweep_staging(&db, &storage).await;
        }
    });
}

/// Completed S3 uploads whose upload URL expired this long ago (or more) get their staging
/// key cleared. A minute past `UPLOAD_TTL`, for clock skew.
const STAGING_SWEEP_AFTER_MINUTES: i64 = storage::UPLOAD_TTL.as_secs() as i64 / 60 + 1;
/// ...and no more than this long ago: wider than the sweep interval, so every file is
/// swept at least once without rescanning old ones forever.
const STAGING_SWEEP_WINDOW_MINUTES: i64 = 25;

/// Deletes what was written to the staging keys of recently completed S3 uploads after they
/// completed: the presigned upload URL keeps working until it expires, but nothing written
/// through it after completion is ever served (see `storage::staging_key`).
async fn sweep_staging(db: &PgPool, storage: &Storage) {
    let files = match File::completed_s3_uploads(db, STAGING_SWEEP_AFTER_MINUTES, STAGING_SWEEP_WINDOW_MINUTES).await {
        Ok(files) => files,
        Err(e) => {
            log::error!("failed to find completed uploads to clean up: {e}");
            return;
        }
    };
    for file in files {
        if let Err(e) = storage.delete_staging(storage::Backend::S3, &file.key).await {
            log::error!("failed to delete the staging object of file {}/{}: {e}", file.owner_id, file.id);
        }
    }
}

/// What the upload sweeper deletes.
#[derive(Clone, Copy)]
enum Sweep {
    /// Uploads still pending after `STALE_UPLOAD_MINUTES`.
    Abandoned,
    /// User files still not attached to anything `UNATTACHED_FILE_HOURS` after uploading.
    Unattached,
}

async fn sweep_files(db: &PgPool, storage: &Storage, sweep: Sweep) {
    let what = match sweep {
        Sweep::Abandoned => "abandoned upload",
        Sweep::Unattached => "unattached file",
    };
    let stale = match sweep {
        Sweep::Abandoned => File::stale_pending(db, STALE_UPLOAD_MINUTES).await,
        Sweep::Unattached => File::stale_unattached(db, UNATTACHED_FILE_HOURS).await,
    };
    let stale = match stale {
        Ok(stale) => stale,
        Err(e) => {
            log::error!("failed to find {what}s: {e}");
            return;
        }
    };
    let mut removed = 0;
    for file in stale {
        // Guarded, so a file completed or attached meanwhile is kept.
        let deleted = match sweep {
            Sweep::Abandoned => File::delete_pending(db, file.owner_id, &file.id).await,
            Sweep::Unattached => File::delete_unattached(db, file.owner_id, &file.id).await,
        };
        match deleted {
            Ok(true) => {}
            Ok(false) => continue,
            Err(e) => {
                log::error!("failed to delete {what} {}/{}: {e}", file.owner_id, file.id);
                continue;
            }
        }
        removed += 1;
        let Some(backend) = storage::Backend::parse(&file.backend) else { continue };
        let result = match sweep {
            // An abandoned upload may have bytes waiting at its staging key.
            Sweep::Abandoned => storage.delete_file(backend, &file.key).await,
            Sweep::Unattached => storage.delete(backend, &file.key).await,
        };
        if let Err(e) = result {
            log::error!("failed to delete object {} of {what}: {e}", file.key);
        }
    }
    if removed > 0 {
        log::info!("deleted {removed} {what}(s)");
    }
}

/// One-shot command mode. Only needs `DATABASE_URL` (and `DATABASE_PASSWORD_FILE`, if
/// used), except `demo` commands, which delete
/// files and close checkouts and so need the full configuration.
async fn run_command(args: &[String]) -> std::io::Result<()> {
    let (db, state) = if args.first().is_some_and(|a| a == "demo") {
        let state = build_state(load_config()).await?;
        (state.db.clone(), Some(state))
    } else {
        let options = config::database_from_env().unwrap_or_else(|e| {
            eprintln!("configuration error: {e}");
            std::process::exit(1);
        });
        (connect_db(options).await?, None)
    };
    match shell::execute(&db, state.as_ref(), args).await {
        Ok(out) => {
            if !out.is_empty() {
                println!("{out}");
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
