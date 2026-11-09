mod auth;
mod config;
mod error;
mod models;
mod routes;
mod shell;
mod state;

use std::{sync::Arc, time::Duration};

use actix_web::{App, HttpServer, middleware::Logger, web};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{auth::jwt::JwtKeys, config::Config, models::Usergroup, state::AppState};

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

    let state = AppState {
        db,
        jwt: JwtKeys::new(&config.jwt_secret),
        config: Arc::new(config.clone()),
        http,
    };

    log::info!("GarryNet backend listening on http://{}", config.bind_addr);
    shell::spawn(state.db.clone());

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

/// Connects to Postgres, applies migrations and creates the default usergroups.
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
    Ok(db)
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
