use actix_web::{HttpResponse, delete, get, post, put, web};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Server,
        server::{AccessKey, ServerDetails},
    },
    routes::validate::{bad, hex_color, optional_url, required_text, text},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_servers)
        .service(get_server)
        .service(create_server)
        .service(update_server)
        .service(delete_server)
        .service(reset_access_key);
}

/// Like the other admin pages, managing servers needs Admin Access as well as the specific
/// permission.
const MANAGE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::SERVER_MANAGEMENT);

/// Every server. Public: servers never include their access key.
#[get("/servers")]
async fn list_servers(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(Server::list(&state.db).await?))
}

/// One server. Public.
#[get("/servers/{id}")]
async fn get_server(
    state: web::Data<AppState>,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let server = Server::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("server"))?;
    Ok(HttpResponse::Ok().json(server))
}

/// A server with its new access key. The only time the key is ever returned.
#[derive(Serialize)]
struct WithAccessKey {
    server: Server,
    access_key: String,
}

/// Registers a server and generates its access key. Requires Admin Access and Server
/// Management.
#[post("/servers")]
async fn create_server(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<ServerBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let details = normalize(body.into_inner())?;
    let key = generate_key()?;
    let server = Server::create(&state.db, &details, &key).await?;
    log::info!("user {} registered server {} ({:?})", user.0.id, server.id, server.name);
    Ok(HttpResponse::Created().json(WithAccessKey {
        server,
        access_key: key.plaintext,
    }))
}

/// Replaces a server's editable details. Requires Admin Access and Server Management.
#[put("/servers/{id}")]
async fn update_server(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<ServerBody>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let details = normalize(body.into_inner())?;
    let server = Server::update(&state.db, id.into_inner(), &details)
        .await?
        .ok_or(ApiError::NotFound("server"))?;
    log::info!("user {} updated server {}", user.0.id, server.id);
    Ok(HttpResponse::Ok().json(server))
}

/// Deletes a server and its player records. Requires Admin Access and Server Management.
#[delete("/servers/{id}")]
async fn delete_server(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let id = id.into_inner();
    if !Server::delete(&state.db, id).await? {
        return Err(ApiError::NotFound("server"));
    }
    log::info!("user {} deleted server {id}", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

/// Generates a new access key and unlinks the server, for when it and GarryNet fall out of
/// sync. Requires Admin Access and Server Management.
#[post("/servers/{id}/access-key")]
async fn reset_access_key(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let key = generate_key()?;
    let server = Server::reset_access_key(&state.db, id.into_inner(), &key)
        .await?
        .ok_or(ApiError::NotFound("server"))?;
    log::info!("user {} regenerated the access key of server {}", user.0.id, server.id);
    Ok(HttpResponse::Ok().json(WithAccessKey {
        server,
        access_key: key.plaintext,
    }))
}

fn generate_key() -> Result<AccessKey, ApiError> {
    AccessKey::generate().map_err(|e| ApiError::Internal(format!("failed to generate access key: {e}")))
}

const DEFAULT_COLOR: &str = "#3b82f6";

#[derive(Deserialize)]
struct ServerBody {
    name: String,
    #[serde(default)]
    description: String,
    splash_url: Option<String>,
    icon_url: Option<String>,
    color: Option<String>,
    #[serde(default)]
    connection_address: String,
}

const MAX_NAME: usize = 64;
const MAX_DESCRIPTION: usize = 2000;
const MAX_ADDRESS: usize = 255;

/// Validates and normalizes (trims, turns blank optional URLs into `None`) a request body.
fn normalize(body: ServerBody) -> Result<ServerDetails, ApiError> {
    let connection_address = text("connection_address", &body.connection_address, MAX_ADDRESS)?;
    if connection_address.chars().any(char::is_whitespace) {
        return Err(bad("connection_address must not contain spaces".to_owned()));
    }
    Ok(ServerDetails {
        name: required_text("name", &body.name, MAX_NAME)?,
        description: text("description", &body.description, MAX_DESCRIPTION)?,
        splash_url: optional_url("splash_url", body.splash_url.as_deref())?,
        icon_url: optional_url("icon_url", body.icon_url.as_deref())?,
        color: match body.color.as_deref().map(str::trim) {
            None | Some("") => DEFAULT_COLOR.to_owned(),
            Some(color) => hex_color("color", color)?,
        },
        connection_address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> ServerBody {
        ServerBody {
            name: " DarkRP ".into(),
            description: String::new(),
            splash_url: Some("".into()),
            icon_url: None,
            color: None,
            connection_address: " 127.0.0.1:27015 ".into(),
        }
    }

    #[test]
    fn normalizes() {
        let d = normalize(body()).unwrap_or_else(|_| panic!("valid body rejected"));
        assert_eq!(d.name, "DarkRP");
        assert_eq!(d.splash_url, None);
        assert_eq!(d.color, DEFAULT_COLOR);
        assert_eq!(d.connection_address, "127.0.0.1:27015");
    }

    #[test]
    fn rejects_bad_input() {
        let mut b = body();
        b.name = " ".into();
        assert!(normalize(b).is_err());

        let mut b = body();
        b.color = Some("blue".into());
        assert!(normalize(b).is_err());

        let mut b = body();
        b.connection_address = "127.0.0.1 27015".into();
        assert!(normalize(b).is_err());

        let mut b = body();
        b.icon_url = Some("ftp://x".into());
        assert!(normalize(b).is_err());
    }
}
