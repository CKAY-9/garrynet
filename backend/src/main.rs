mod auth;
mod config;
mod error;
mod models;
mod notify;
mod routes;
mod shell;
mod state;
mod storage;
mod stripe;

use std::{sync::Arc, time::Duration};

use actix_web::{App, HttpServer, middleware::Logger, web};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    auth::jwt::JwtKeys,
    config::Config,
    models::{
        Community, File, Usergroup, notification::Notification,
        support::SupportTicket,
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

    let config = Config::from_env().unwrap_or_else(|e| {
        eprintln!("configuration error: {e}");
        std::process::exit(1);
    });

    let db = connect_db(&config.database_url).await?;

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(std::io::Error::other)?;

    let stripe = config.stripe.clone().map(|stripe| Stripe::new(http.clone(), stripe));
    if stripe.is_none() {
        log::warn!("STRIPE_SECRET_KEY and STRIPE_WEBHOOK_SECRET are not set: the store can't take payments");
    }

    let storage = Storage::new(&config.storage, &config.backend_url, &config.jwt_secret)
        .unwrap_or_else(|e| {
            eprintln!("configuration error: {e}");
            std::process::exit(1);
        });
    log::info!("storing new uploads with the {} provider", storage.default_backend());

    let state = AppState {
        db,
        jwt: JwtKeys::new(&config.jwt_secret),
        config: Arc::new(config.clone()),
        http,
        stripe,
        storage,
    };

    log::info!("GarryNet backend listening on http://{}", config.bind_addr);
    shell::spawn(state.db.clone());
    spawn_ticket_auto_close(state.db.clone());
    spawn_purchase_sweeper(state.clone());
    spawn_upload_sweeper(state.db.clone(), state.storage.clone());
    spawn_notification_sweeper(state.db.clone());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::new("%r %s %Dms"))
            .configure(routes::configure)
    })
    .bind(&config.bind_addr)?
    .run()
    .await
}

/// Connects to Postgres, applies migrations and creates the default usergroups and the
/// skeleton community.
async fn connect_db(database_url: &str) -> std::io::Result<PgPool> {
    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
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

/// Settles store purchases whose Checkout Session should long have been settled by a
/// webhook (see `routes::store::sweep_stale_purchases`). Every 10 minutes.
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
        }
    });
}

/// Upload targets expire after 15 minutes; a file still pending this long was abandoned.
const STALE_UPLOAD_MINUTES: i64 = 60;

/// Deletes abandoned uploads (rows and any bytes that did arrive), every 10 minutes.
fn spawn_upload_sweeper(db: PgPool, storage: Storage) {
    actix_web::rt::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(10 * 60));
        loop {
            tick.tick().await;
            let stale = match File::stale_pending(&db, STALE_UPLOAD_MINUTES).await {
                Ok(stale) => stale,
                Err(e) => {
                    log::error!("failed to find abandoned uploads: {e}");
                    continue;
                }
            };
            let mut removed = 0;
            for file in stale {
                match File::delete_pending(&db, file.owner_id, &file.id).await {
                    Ok(true) => {}
                    Ok(false) => continue, // completed meanwhile
                    Err(e) => {
                        log::error!("failed to delete abandoned upload {}/{}: {e}", file.owner_id, file.id);
                        continue;
                    }
                }
                removed += 1;
                let Some(backend) = storage::Backend::parse(&file.backend) else { continue };
                if let Err(e) = storage.delete(backend, &file.key).await {
                    log::error!("failed to delete object {} of abandoned upload: {e}", file.key);
                }
            }
            if removed > 0 {
                log::info!("deleted {removed} abandoned upload(s)");
            }
        }
    });
}

/// One-shot command mode. Only needs `DATABASE_URL`.
async fn run_command(args: &[String]) -> std::io::Result<()> {
    let Some(database_url) = std::env::var("DATABASE_URL").ok().filter(|v| !v.is_empty())
    else {
        eprintln!("configuration error: missing required environment variable DATABASE_URL");
        std::process::exit(1);
    };
    let db = connect_db(&database_url).await?;
    match shell::execute(&db, args).await {
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
