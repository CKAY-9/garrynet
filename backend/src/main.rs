mod auth;
mod config;
mod error;
mod models;
mod routes;
mod shell;
mod state;
mod stripe;

use std::{sync::Arc, time::Duration};

use actix_web::{App, HttpServer, middleware::Logger, web};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    auth::jwt::JwtKeys,
    config::Config,
    models::{Community, Usergroup, store::Purchase, support::SupportTicket},
    state::AppState,
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

    let state = AppState {
        db,
        jwt: JwtKeys::new(&config.jwt_secret),
        config: Arc::new(config.clone()),
        http,
        stripe,
    };

    log::info!("GarryNet backend listening on http://{}", config.bind_addr);
    shell::spawn(state.db.clone());
    spawn_ticket_auto_close(state.db.clone());
    spawn_purchase_sweeper(state.db.clone());

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
                Ok(0) => {}
                Ok(n) => log::info!("automatically closed {n} inactive support ticket(s)"),
                Err(e) => log::error!("failed to auto-close support tickets: {e}"),
            }
        }
    });
}

/// Checkout Sessions expire after 30 minutes; a pending purchase this old has missed its
/// `checkout.session.expired` webhook.
const STALE_PURCHASE_MINUTES: i64 = 90;

/// Cancels store purchases whose Checkout Session has long expired without Stripe telling
/// us, returning their reserved stock. Every 10 minutes. A purchase that was in fact paid is
/// reinstated when its webhook finally arrives (see `Purchase::mark_paid`).
fn spawn_purchase_sweeper(db: PgPool) {
    actix_web::rt::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(10 * 60));
        loop {
            tick.tick().await;
            match Purchase::cancel_stale(&db, STALE_PURCHASE_MINUTES).await {
                Ok(cancelled) if cancelled.is_empty() => {}
                Ok(cancelled) => log::info!("cancelled {} stale store purchase(s)", cancelled.len()),
                Err(e) => log::error!("failed to sweep stale store purchases: {e}"),
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
