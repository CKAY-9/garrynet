use std::sync::Arc;

use sqlx::PgPool;

use crate::{auth::jwt::JwtKeys, config::Config};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
    pub jwt: JwtKeys,
}
