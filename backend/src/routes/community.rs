use actix_web::{HttpResponse, get, post, put, web};
use serde::Deserialize;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Community, Permissions, User,
        community::{CommunityDetails, KeyFigure},
    },
    routes::validate::{bad, optional_url, required_text, text, url},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_community)
        .service(create_community)
        .service(update_community);
}

/// The community. Public: every page's navigation shows its name and logo.
#[get("/community")]
async fn get_community(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // Startup creates the community; recreate the skeleton if it has since been deleted.
    let community = match Community::get(&state.db).await? {
        Some(community) => community,
        None => {
            Community::ensure_exists(&state.db).await?;
            Community::get(&state.db)
                .await?
                .ok_or(ApiError::NotFound("community"))?
        }
    };
    Ok(HttpResponse::Ok().json(community))
}

/// Creates the community. Startup already creates a skeleton, so this only succeeds if it
/// has been deleted; otherwise 409. Requires Admin Access and Community Management.
#[post("/community")]
async fn create_community(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<CommunityBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let details = validate(&state, body.into_inner()).await?;
    let community = Community::create(&state.db, &details)
        .await?
        .ok_or(ApiError::Conflict("community"))?;
    log::info!("user {} created the community", user.0.id);
    Ok(HttpResponse::Created().json(community))
}

/// Replaces the community's details. Requires Admin Access and Community Management.
#[put("/community")]
async fn update_community(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<CommunityBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let details = validate(&state, body.into_inner()).await?;
    let community = Community::update(&state.db, &details)
        .await?
        .ok_or(ApiError::NotFound("community"))?;
    log::info!("user {} updated the community details", user.0.id);
    Ok(HttpResponse::Ok().json(community))
}

/// Like the other admin pages, editing needs Admin Access as well as the specific permission.
const MANAGE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::COMMUNITY_MANAGEMENT);

#[derive(Deserialize)]
struct CommunityBody {
    name: String,
    #[serde(default)]
    description: String,
    logo_url: Option<String>,
    splash_background: Option<String>,
    #[serde(default)]
    external_links: Vec<String>,
    #[serde(default)]
    key_figures: Vec<KeyFigure>,
}

const MAX_NAME: usize = 64;
const MAX_DESCRIPTION: usize = 4000;
const MAX_LINKS: usize = 16;
const MAX_KEY_FIGURES: usize = 32;
const MAX_ROLE: usize = 64;
const MAX_FIGURE_DESCRIPTION: usize = 500;

/// Validates and normalizes (trims, turns blank optional URLs into `None`) a request
/// body, and checks that key figures' user ids exist.
async fn validate(state: &AppState, body: CommunityBody) -> Result<CommunityDetails, ApiError> {
    let details = normalize(body)?;
    let ids: Vec<i64> = details.key_figures.iter().filter_map(|f| f.user_id).collect();
    if !ids.is_empty()
        && let Some(id) = User::missing_ids(&state.db, &ids).await?.first()
    {
        return Err(bad(format!("key figure user {id} does not exist")));
    }
    Ok(details)
}

fn normalize(body: CommunityBody) -> Result<CommunityDetails, ApiError> {
    let name = required_text("name", &body.name, MAX_NAME)?;
    let description = text("description", &body.description, MAX_DESCRIPTION)?;
    let logo_url = optional_url("logo_url", body.logo_url.as_deref())?;
    let splash_background = optional_url("splash_background", body.splash_background.as_deref())?;

    if body.external_links.len() > MAX_LINKS {
        return Err(bad(format!("at most {MAX_LINKS} external links are allowed")));
    }
    let external_links = body
        .external_links
        .iter()
        .map(|link| url("external_links", link))
        .collect::<Result<_, _>>()?;

    if body.key_figures.len() > MAX_KEY_FIGURES {
        return Err(bad(format!("at most {MAX_KEY_FIGURES} key figures are allowed")));
    }
    let key_figures = body
        .key_figures
        .iter()
        .map(|f| {
            Ok(KeyFigure {
                user_id: f.user_id,
                name: required_text("key figure name", &f.name, MAX_NAME)?,
                profile_picture: optional_url("key figure profile_picture", Some(&f.profile_picture))?
                    .unwrap_or_default(),
                role: text("key figure role", &f.role, MAX_ROLE)?,
                description: text("key figure description", &f.description, MAX_FIGURE_DESCRIPTION)?,
            })
        })
        .collect::<Result<_, ApiError>>()?;

    Ok(CommunityDetails {
        name,
        description,
        logo_url,
        splash_background,
        external_links,
        key_figures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> CommunityBody {
        CommunityBody {
            name: "  Test  ".into(),
            description: String::new(),
            logo_url: Some(" ".into()),
            splash_background: None,
            external_links: vec!["https://discord.gg/x".into()],
            key_figures: vec![KeyFigure {
                user_id: None,
                name: "A".into(),
                profile_picture: String::new(),
                role: "Owner".into(),
                description: String::new(),
            }],
        }
    }

    #[test]
    fn normalizes() {
        let details = normalize(body()).unwrap_or_else(|_| panic!("valid body rejected"));
        assert_eq!(details.name, "Test");
        assert_eq!(details.logo_url, None);
        assert_eq!(details.key_figures[0].profile_picture, "");
    }

    #[test]
    fn rejects_bad_input() {
        let mut b = body();
        b.name = "   ".into();
        assert!(normalize(b).is_err());

        let mut b = body();
        b.external_links = vec!["javascript:alert(1)".into()];
        assert!(normalize(b).is_err());

        let mut b = body();
        b.logo_url = Some("not a url".into());
        assert!(normalize(b).is_err());

        let mut b = body();
        b.key_figures[0].name = String::new();
        assert!(normalize(b).is_err());
    }
}
