use std::sync::Arc;

use sqlx::PgPool;

use crate::{auth::jwt::JwtKeys, config::Config, steam_profiles::SteamProfiles, storage::Storage, stripe::Stripe};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    pub http: reqwest::Client,
    pub jwt: JwtKeys,
    /// `None` until Stripe is configured (docs/STORE.md); nothing can be bought then.
    pub stripe: Option<Stripe>,
    /// File storage (docs/STORAGE.md).
    pub storage: Storage,
    /// Steam profiles looked up for the loading screen.
    pub steam_profiles: Arc<SteamProfiles>,
}

impl AppState {
    /// The Stripe client, or 503 if this deployment hasn't configured Stripe.
    pub fn stripe(&self) -> Result<&Stripe, crate::error::ApiError> {
        self.stripe
            .as_ref()
            .ok_or(crate::error::ApiError::NotConfigured("the store's payment provider (Stripe)"))
    }
}
