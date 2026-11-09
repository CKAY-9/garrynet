use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "loading_screen_style", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum LoadingScreenStyle {
    /// Frutiger Aero, with the feel of a classic Garry's Mod loading screen.
    #[default]
    Aero,
    /// Minimalist, clean and dim.
    Sleek,
}

/// The community's loading screen settings, shared by every server.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct LoadingScreenConfig {
    pub id: i16,
    pub style: LoadingScreenStyle,
}

/// The table always has exactly this one row.
const CONFIG_ID: i16 = 1;

impl LoadingScreenConfig {
    pub async fn get(db: &PgPool) -> sqlx::Result<Self> {
        let config = sqlx::query_as("SELECT id, style FROM loading_screen_config WHERE id = $1")
            .bind(CONFIG_ID)
            .fetch_optional(db)
            .await?;
        // The migration creates the row, but fall back to the defaults if it's gone.
        Ok(config.unwrap_or(Self {
            id: CONFIG_ID,
            style: LoadingScreenStyle::default(),
        }))
    }

    pub async fn set_style(db: &PgPool, style: LoadingScreenStyle) -> sqlx::Result<Self> {
        sqlx::query_as(
            "INSERT INTO loading_screen_config (id, style) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET style = EXCLUDED.style
             RETURNING id, style",
        )
        .bind(CONFIG_ID)
        .bind(style)
        .fetch_one(db)
        .await
    }
}
