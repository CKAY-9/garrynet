use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, types::Json};

/// A notable member of the community, shown on the homepage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyFigure {
    /// Their GarryNet account, if they have one; the homepage links to their profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
    pub name: String,
    /// Image URL; empty for none.
    pub profile_picture: String,
    pub role: String,
    pub description: String,
}

/// The community this deployment serves. There is exactly one.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Community {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub logo_url: Option<String>,
    /// Image URL shown behind the homepage header.
    pub splash_background: Option<String>,
    pub external_links: Vec<String>,
    pub key_figures: Json<Vec<KeyFigure>>,
    /// Whether the addon keeps users banned on GarryNet off the community's game servers.
    pub ban_from_servers: bool,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// The editable parts of the community. Already validated (see `routes::community`).
pub struct CommunityDetails {
    pub name: String,
    pub description: String,
    pub logo_url: Option<String>,
    pub splash_background: Option<String>,
    pub external_links: Vec<String>,
    pub key_figures: Vec<KeyFigure>,
    pub ban_from_servers: bool,
}

impl CommunityDetails {
    /// The placeholder community created on first start, to be edited in the admin dashboard.
    pub fn skeleton() -> Self {
        Self {
            name: "GarryNet Community".to_owned(),
            description: "Set your own community description in the admin dashboard".to_owned(),
            logo_url: None,
            splash_background: None,
            external_links: Vec::new(),
            key_figures: vec![KeyFigure {
                user_id: None,
                name: "CKAY9".to_owned(),
                profile_picture: "https://avatars.githubusercontent.com/u/53030585?v=4".to_owned(),
                role: "GarryNet Developer".to_owned(),
                description: "Hey, this is the thing I made.".to_owned(),
            }],
            ban_from_servers: false,
        }
    }
}

/// `$sql` followed by the community's columns, for `RETURNING` / `SELECT`.
macro_rules! with_columns {
    ($sql:literal $(, $rest:literal)?) => {
        concat!(
            $sql,
            " id, name, description, logo_url, splash_background, external_links,
              key_figures, ban_from_servers, created_at, last_updated"
            $(, $rest)?
        )
    };
}

impl Community {
    /// The community, or `None` if it hasn't been created.
    pub async fn get(db: &PgPool) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(with_columns!("SELECT", " FROM community LIMIT 1"))
            .fetch_optional(db)
            .await
    }

    /// Whether users banned on GarryNet are kept off the game servers. `false` if the
    /// community hasn't been created.
    pub async fn bans_from_servers(db: &PgPool) -> sqlx::Result<bool> {
        Ok(sqlx::query_scalar("SELECT ban_from_servers FROM community LIMIT 1")
            .fetch_optional(db)
            .await?
            .unwrap_or(false))
    }

    /// Creates the community, or returns `None` if one already exists.
    pub async fn create(db: &PgPool, details: &CommunityDetails) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(with_columns!(
            "INSERT INTO community
                 (name, description, logo_url, splash_background, external_links, key_figures,
                  ban_from_servers)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT DO NOTHING
             RETURNING"
        ))
        .bind(&details.name)
        .bind(&details.description)
        .bind(&details.logo_url)
        .bind(&details.splash_background)
        .bind(&details.external_links)
        .bind(Json(&details.key_figures))
        .bind(details.ban_from_servers)
        .fetch_optional(db)
        .await
    }

    /// Creates the skeleton community if there is none yet. An existing community is left
    /// untouched, so edits survive restarts.
    pub async fn ensure_exists(db: &PgPool) -> sqlx::Result<()> {
        if let Some(community) = Self::create(db, &CommunityDetails::skeleton()).await? {
            log::info!(
                "created skeleton community (id {}); edit it at /admin/community",
                community.id
            );
        }
        Ok(())
    }

    /// Replaces the community's details, or returns `None` if it hasn't been created.
    pub async fn update(db: &PgPool, details: &CommunityDetails) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(with_columns!(
            "UPDATE community
             SET name = $1, description = $2, logo_url = $3, splash_background = $4,
                 external_links = $5, key_figures = $6, ban_from_servers = $7,
                 last_updated = now()
             RETURNING"
        ))
        .bind(&details.name)
        .bind(&details.description)
        .bind(&details.logo_url)
        .bind(&details.splash_background)
        .bind(&details.external_links)
        .bind(Json(&details.key_figures))
        .bind(details.ban_from_servers)
        .fetch_optional(db)
        .await
    }
}
