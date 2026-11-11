//! File uploads and downloads (docs/STORAGE.md).
//!
//! Uploading takes three requests: `POST /file/upload` records the file and returns where
//! to send its bytes, the client sends them there (a presigned S3 URL, or
//! `PUT /file/{owner}/{id}/content` for the local provider), then
//! `POST /file/{owner}/{id}/complete` verifies them and makes the file readable.

use actix_web::{
    FromRequest, HttpRequest, HttpResponse, delete, get,
    http::header::{self, ContentDisposition, DispositionParam, DispositionType, EntityTag},
    post, put, route, web,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        File, Permissions,
        file::{COMMUNITY_OWNER, FileStatus, NewFile},
        support::SupportMessage,
    },
    routes::validate::bad,
    state::AppState,
    storage::{Backend, ReadTarget, UploadTarget},
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(create_upload)
        .service(upload_content_preflight)
        .service(upload_content)
        .service(complete_upload)
        .service(get_file)
        .service(delete_file);
}

const UPLOAD: Permissions = Permissions::SITE_ACCESS;
/// Community files are the images set in the admin pages (logos, icons, splashes), which
/// are managed by different permissions, so uploading one only needs the dashboard itself.
const UPLOAD_COMMUNITY: Permissions = Permissions::ADMIN_ACCESS;
const MANAGE_COMMUNITY: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::COMMUNITY_MANAGEMENT);
/// Deleting other users' files.
const MANAGE_USERS: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::USER_MANAGEMENT);
/// Reading any support attachment, as `routes::support` lets the team read any ticket.
const SUPPORT_TEAM: Permissions = Permissions::SUPPORT_TEAM;

/// Content types that can be uploaded. Nothing a browser would run as a document (HTML,
/// SVG, ...): files are served from the backend's own origin.
const ALLOWED_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/avif",
    "audio/mpeg",
    "audio/ogg",
    "audio/wav",
    "video/mp4",
    "video/webm",
    "application/pdf",
    "text/plain",
];

const MAX_NAME: usize = 255;

/// Uploads a user can start back to back before the rate limit spaces them out: enough
/// for the most attachments one message can have. Over time the rate is still one upload
/// per `STORAGE_RATE_LIMIT`.
const UPLOAD_BURST: i64 = 10;

/// A file as clients see it.
#[derive(Serialize)]
struct FileView {
    owner_id: i64,
    id: String,
    /// Where to fetch it; stable across storage providers.
    url: String,
    name: String,
    content_type: String,
    size: i64,
    status: FileStatus,
    uploaded_by: Option<i64>,
    created_at: chrono::DateTime<Utc>,
    completed_at: Option<chrono::DateTime<Utc>>,
}

impl FileView {
    fn new(state: &AppState, file: File) -> Self {
        Self {
            url: format!("{}/file/{}/{}", state.config.backend_url, file.owner_id, file.id),
            owner_id: file.owner_id,
            id: file.id,
            name: file.name,
            content_type: file.content_type,
            size: file.size,
            status: file.status,
            uploaded_by: file.uploaded_by,
            created_at: file.created_at,
            completed_at: file.completed_at,
        }
    }
}

#[derive(Deserialize)]
struct UploadBody {
    /// The client's file name; only used as the download name.
    name: String,
    content_type: String,
    /// In bytes.
    size: u64,
    /// Store it as a community file (`/file/-1/...`) instead of in the user's folder.
    #[serde(default)]
    community: bool,
}

#[derive(Serialize)]
struct UploadResponse {
    file: FileView,
    upload: UploadTarget,
}

/// Starts an upload: records a pending file and returns where to send its bytes. Requires
/// Site Access; community files require Admin Access.
#[post("/file/upload")]
async fn create_upload(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<UploadBody>,
) -> Result<HttpResponse, ApiError> {
    let body = body.into_inner();
    let storage = &state.storage;
    user.require(&state.db, if body.community { UPLOAD_COMMUNITY } else { UPLOAD })
        .await?;

    let content_type = content_type(&body.content_type)?;
    let max = storage.max_file_size();
    if body.size == 0 {
        return Err(bad("size must be greater than 0".into()));
    }
    if body.size > max {
        return Err(bad(format!("files can be at most {max} bytes")));
    }
    let name = file_name(&body.name);

    // At most UPLOAD_BURST uploads in any window of UPLOAD_BURST rate-limit periods.
    let window = storage.rate_limit().as_secs_f64() * UPLOAD_BURST as f64;
    if window > 0.0 {
        let (count, oldest) = File::recent_uploads_by(&state.db, user.0.id, window).await?;
        if count >= UPLOAD_BURST {
            let elapsed = oldest.map_or(0.0, |t| (Utc::now() - t).num_milliseconds() as f64 / 1000.0);
            return Err(ApiError::RateLimited((window - elapsed).ceil().max(1.0) as u64));
        }
    }

    let owner_id = if body.community { COMMUNITY_OWNER } else { user.0.id };
    let id = new_file_id()?;
    let key = File::key_for(owner_id, &id);
    let backend = storage.default_backend();
    let file = File::create(
        &state.db,
        &NewFile {
            owner_id,
            id: &id,
            backend: backend.as_str(),
            key: &key,
            name: &name,
            content_type: &content_type,
            size: body.size as i64,
            uploaded_by: user.0.id,
        },
    )
    .await?;
    let upload = storage
        .create_upload(backend, owner_id, &id, &key, &content_type)
        .await?;
    Ok(HttpResponse::Created().json(UploadResponse { file: FileView::new(&state, file), upload }))
}

#[derive(Deserialize)]
struct UploadQuery {
    expires: i64,
    signature: String,
}

/// CORS preflight for [`upload_content`]: browsers on the frontend upload to it directly.
#[route("/file/{owner_id}/{file_id}/content", method = "OPTIONS")]
async fn upload_content_preflight(state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::NoContent()
        .insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, state.config.frontend_url.clone()))
        .insert_header((header::ACCESS_CONTROL_ALLOW_METHODS, "PUT"))
        .insert_header((header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type"))
        .insert_header((header::ACCESS_CONTROL_MAX_AGE, "600"))
        .insert_header((header::VARY, "Origin"))
        .finish()
}

/// Receives the bytes of a pending local-provider file and completes it. Authorized by the
/// signature in the URL issued by `POST /file/upload`, not by login, so it's safe to let
/// the frontend's origin call it (no cookies are involved).
#[put("/file/{owner_id}/{file_id}/content")]
async fn upload_content(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(i64, String)>,
    query: web::Query<UploadQuery>,
    body: web::Payload,
) -> Result<HttpResponse, ApiError> {
    let mut response = receive_content(&state, &req, path.into_inner(), query.into_inner(), body)
        .await
        .unwrap_or_else(|e| actix_web::ResponseError::error_response(&e));
    response.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_str(&state.config.frontend_url)
            .map_err(|_| ApiError::Internal("FRONTEND_URL isn't a valid header value".into()))?,
    );
    Ok(response)
}

async fn receive_content(
    state: &AppState,
    req: &HttpRequest,
    (owner_id, file_id): (i64, String),
    query: UploadQuery,
    body: web::Payload,
) -> Result<HttpResponse, ApiError> {
    let storage = &state.storage;
    if !storage.verify_upload(owner_id, &file_id, query.expires, &query.signature) {
        return Err(ApiError::Forbidden("a valid, unexpired upload URL".into()));
    }
    let file = File::get(&state.db, owner_id, &file_id)
        .await?
        .ok_or(ApiError::NotFound("file"))?;
    if file.status != FileStatus::Pending {
        return Err(ApiError::Conflict("file"));
    }
    let backend = backend_of(&file)?;
    if backend != Backend::Local {
        return Err(bad("this file is uploaded to its storage provider directly".into()));
    }
    let sent_type = req
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if sent_type != file.content_type {
        return Err(bad(format!("Content-Type must be {}", file.content_type)));
    }
    let declared = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok());
    if declared.is_some_and(|len| len != file.size) {
        return Err(bad(format!("Content-Length must be {}", file.size)));
    }

    storage.write(backend, &file.key, file.size as u64, body).await?;
    complete(state, file, backend).await
}

/// Verifies a pending file's bytes arrived and makes it readable. Only its uploader may
/// complete it. Completing a ready file just returns it.
#[post("/file/{owner_id}/{file_id}/complete")]
async fn complete_upload(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, String)>,
) -> Result<HttpResponse, ApiError> {
    let (owner_id, file_id) = path.into_inner();
    let file = File::get(&state.db, owner_id, &file_id)
        .await?
        .filter(|file| file.uploaded_by == Some(user.0.id))
        .ok_or(ApiError::NotFound("file"))?;
    if file.status == FileStatus::Ready {
        return Ok(HttpResponse::Ok().json(FileView::new(&state, file)));
    }
    let backend = backend_of(&file)?;
    complete(&state, file, backend).await
}

/// Checks the stored object matches what was declared, then marks the file ready.
async fn complete(state: &AppState, file: File, backend: Backend) -> Result<HttpResponse, ApiError> {
    let storage = &state.storage;
    let Some((size, e_tag)) = storage.head(backend, &file.key).await? else {
        return Err(bad("the file's bytes haven't been uploaded yet".into()));
    };
    if size != file.size as u64 {
        // A presigned URL can't limit the size, so a client could have sent anything.
        storage.delete(backend, &file.key).await?;
        return Err(bad(format!(
            "uploaded {size} bytes but {} were declared; upload it again",
            file.size
        )));
    }
    match File::mark_ready(&state.db, file.owner_id, &file.id, e_tag.as_deref()).await? {
        Some(file) => Ok(HttpResponse::Ok().json(FileView::new(state, file))),
        // Deleted (or swept) while uploading.
        None => {
            storage.delete(backend, &file.key).await?;
            Err(ApiError::NotFound("file"))
        }
    }
}

/// Serves a file: streamed from local storage, or a redirect to the cloud provider. Public
/// (file IDs are unguessable), except files attached to a support message: those are only
/// served to the ticket's creator (not for internal notes) and the Support Team, by the
/// auth cookie or token, and anyone else gets 404.
#[get("/file/{owner_id}/{file_id}")]
async fn get_file(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(i64, String)>,
) -> Result<HttpResponse, ApiError> {
    let (owner_id, file_id) = path.into_inner();
    let file = File::get(&state.db, owner_id, &file_id)
        .await?
        .filter(|file| file.status == FileStatus::Ready)
        .ok_or(ApiError::NotFound("file"))?;
    let private = file.support_message_id.is_some();
    if let Some(message_id) = file.support_message_id
        && !may_read_support_file(&state, &req, message_id).await?
    {
        return Err(ApiError::NotFound("file"));
    }
    let backend = backend_of(&file)?;
    let storage = &state.storage;

    match storage.read_target(backend, &file.key, private).await? {
        ReadTarget::Redirect(url) => Ok(HttpResponse::Found()
            .insert_header((header::LOCATION, url))
            // Presigned URLs expire, so don't let the redirect outlive them; a private one
            // isn't kept at all, so every view is checked again.
            .insert_header((header::CACHE_CONTROL, if private { "private, no-store" } else { "private, max-age=300" }))
            .finish()),
        ReadTarget::Stream => {
            // Files never change once ready, so the ID is a strong validator.
            let etag = EntityTag::new_strong(file.id.clone());
            let not_modified = req
                .headers()
                .get(header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.split(',').any(|t| t.trim().trim_start_matches("W/") == etag.to_string()));
            let mut response = if not_modified {
                HttpResponse::NotModified()
            } else {
                HttpResponse::Ok()
            };
            response
                .insert_header((header::ETAG, etag))
                // A private file may only sit in the viewer's own cache, and is revalidated
                // (a cheap 304) on every use, so losing access takes effect at once.
                .insert_header((
                    header::CACHE_CONTROL,
                    if private { "private, no-cache" } else { "public, max-age=31536000, immutable" },
                ))
                .insert_header((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
                .insert_header((header::CONTENT_SECURITY_POLICY, "default-src 'none'; sandbox"));
            if not_modified {
                return Ok(response.finish());
            }
            let stream = storage
                .get(backend, &file.key)
                .await?
                .ok_or(ApiError::NotFound("file"))?;
            Ok(response
                .insert_header((header::CONTENT_TYPE, file.content_type))
                .insert_header(ContentDisposition {
                    disposition: DispositionType::Inline,
                    parameters: vec![DispositionParam::Filename(file.name)],
                })
                .no_chunking(file.size as u64)
                .streaming(stream))
        }
    }
}

/// Whether the requester may read a file attached to support message `message_id`: the
/// ticket's creator (unless it's an internal note) or the Support Team. The login is only
/// looked up here, so public files don't mark their viewers online.
async fn may_read_support_file(state: &AppState, req: &HttpRequest, message_id: i64) -> Result<bool, ApiError> {
    let Some(audience) = SupportMessage::audience(&state.db, message_id).await? else {
        return Ok(false);
    };
    let user = match AuthUser::extract(req).await {
        Ok(user) => user,
        Err(ApiError::Unauthorized) => return Ok(false),
        Err(e) => return Err(e),
    };
    if audience.visible_to(user.0.id) {
        return Ok(true);
    }
    Ok(user.require(&state.db, SUPPORT_TEAM).await.is_ok())
}

/// Validates the files a user wants to attach to something they're posting: at most `max`
/// of them, each a ready, not yet attached file in their own folder. Returns their IDs
/// without duplicates, for `File::attach`.
pub async fn attachable(state: &AppState, user_id: i64, ids: &[String], max: usize) -> Result<Vec<String>, ApiError> {
    let mut unique: Vec<String> = Vec::with_capacity(ids.len());
    for id in ids {
        if !unique.contains(id) {
            unique.push(id.clone());
        }
    }
    if unique.len() > max {
        return Err(bad(format!("at most {max} files can be attached")));
    }
    let valid = unique.iter().all(|id| id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()));
    if !valid || !File::attachable(&state.db, user_id, &unique).await? {
        return Err(bad(
            "attachments must be files you uploaded that aren't attached to anything else".into(),
        ));
    }
    Ok(unique)
}

/// Deletes files whose post or message is gone: their rows, then their bytes. Failures are
/// logged, not returned; what they were attached to is already deleted.
pub async fn delete_all(state: &AppState, files: Vec<File>) {
    for file in files {
        let result = async {
            if File::delete(&state.db, file.owner_id, &file.id).await? {
                state.storage.delete(backend_of(&file)?, &file.key).await?;
            }
            Ok::<_, ApiError>(())
        }
        .await;
        if let Err(e) = result {
            log::error!("failed to delete file {}/{}: {e}", file.owner_id, file.id);
        }
    }
}

/// Deletes a file. Allowed for its uploader, users with Admin Access and User Management
/// (user files) or Admin Access and Community Management (community files).
#[delete("/file/{owner_id}/{file_id}")]
async fn delete_file(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, String)>,
) -> Result<HttpResponse, ApiError> {
    let (owner_id, file_id) = path.into_inner();
    let file = File::get(&state.db, owner_id, &file_id)
        .await?
        .ok_or(ApiError::NotFound("file"))?;
    if file.uploaded_by != Some(user.0.id) {
        let required = if owner_id == COMMUNITY_OWNER { MANAGE_COMMUNITY } else { MANAGE_USERS };
        user.require(&state.db, required).await?;
    }
    let backend = backend_of(&file)?;
    if File::delete(&state.db, owner_id, &file_id).await? {
        state.storage.delete(backend, &file.key).await?;
        log::info!("user {} deleted file {owner_id}/{file_id}", user.0.id);
    }
    Ok(HttpResponse::NoContent().finish())
}

fn backend_of(file: &File) -> Result<Backend, ApiError> {
    Backend::parse(&file.backend)
        .ok_or_else(|| ApiError::Internal(format!("file {}/{} has unknown backend {:?}", file.owner_id, file.id, file.backend)))
}

/// 32 random hex characters.
fn new_file_id() -> Result<String, ApiError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::Internal(format!("getrandom: {e}")))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// The content type without parameters, lowercased, if it's allowed.
fn content_type(value: &str) -> Result<String, ApiError> {
    let essence = value.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
    if ALLOWED_TYPES.contains(&essence.as_str()) {
        Ok(essence)
    } else {
        Err(bad(format!(
            "content_type {value:?} isn't allowed; use one of {}",
            ALLOWED_TYPES.join(", ")
        )))
    }
}

/// The last path component of a client file name, without control characters or quotes,
/// at most [`MAX_NAME`] characters. Never used in storage keys.
fn file_name(value: &str) -> String {
    let base = value.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base
        .chars()
        .filter(|c| !c.is_control() && *c != '"')
        .take(MAX_NAME)
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        "file".to_owned()
    } else {
        cleaned.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_types() {
        assert_eq!(content_type("image/PNG").unwrap(), "image/png");
        assert_eq!(content_type("text/plain; charset=utf-8").unwrap(), "text/plain");
        assert!(content_type("image/svg+xml").is_err());
        assert!(content_type("text/html").is_err());
        assert!(content_type("").is_err());
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name("cat.png"), "cat.png");
        assert_eq!(file_name("../../etc/passwd"), "passwd");
        assert_eq!(file_name("C:\\Users\\me\\a \"b\".png"), "a b.png");
        assert_eq!(file_name(".."), "file");
        assert_eq!(file_name("  "), "file");
        assert_eq!(file_name(&"a".repeat(300)).len(), MAX_NAME);
    }

    #[test]
    fn file_ids() {
        let id = new_file_id().unwrap();
        assert_eq!(id.len(), 32);
        assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(id, new_file_id().unwrap());
    }
}
