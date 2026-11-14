//! File storage behind one interface (docs/STORAGE.md).
//!
//! Every provider is an [`ObjectStore`]; [`Storage`] adds what differs between them:
//! how a client uploads (a presigned S3 URL, or a signed route on this backend that streams
//! to disk) and how a client reads (a redirect, or a stream from this backend).
//!
//! Files record the provider they were uploaded to, so switching `STORAGE_PROVIDER` only
//! affects new uploads; old files keep resolving as long as their provider stays configured.

use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use actix_web::web::Bytes;
use chrono::{DateTime, Utc};
use futures_util::{Stream, StreamExt};
use hmac::{Hmac, Mac};
use object_store::{
    ObjectStore, ObjectStoreExt, WriteMultipart,
    aws::{AmazonS3, AmazonS3Builder},
    local::LocalFileSystem,
    path::Path,
    signer::{HeaderName, HeaderValue, Method, SignedUrlOptions, Signer},
};
use serde::Serialize;
use sha2::Sha256;

use crate::{
    config::{StorageConfig, StorageProvider},
    error::ApiError,
};

/// How long an issued upload target stays valid.
pub const UPLOAD_TTL: Duration = Duration::from_secs(15 * 60);
/// How long a presigned read URL stays valid.
const READ_TTL: Duration = Duration::from_secs(60 * 60);

/// Name of a provider, as stored in `files.backend`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Backend {
    Local,
    S3,
}

impl Backend {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "local" => Some(Self::Local),
            "s3" => Some(Self::S3),
            _ => None,
        }
    }
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

enum Provider {
    Local(Arc<LocalFileSystem>),
    S3 {
        store: Arc<AmazonS3>,
        public_url: Option<String>,
    },
}

impl Provider {
    fn store(&self) -> &dyn ObjectStore {
        match self {
            Self::Local(store) => store.as_ref(),
            Self::S3 { store, .. } => store.as_ref(),
        }
    }
}

/// Where and how the client sends the file's bytes.
#[derive(Debug, Serialize)]
pub struct UploadTarget {
    pub method: &'static str,
    pub url: String,
    /// Headers the request must carry, exactly as given.
    pub headers: HashMap<&'static str, String>,
    pub expires_at: DateTime<Utc>,
}

/// What [`Storage::finish_upload`] found.
pub enum Finish {
    /// Nothing has been uploaded yet.
    Missing,
    /// The upload had this many bytes, not the declared size; it was deleted.
    WrongSize(u64),
    /// The bytes are in place at the file's key, with this ETag.
    Done(Option<String>),
}

/// Where the bytes of an S3 upload are sent before they're checked. A presigned PUT URL
/// can't limit the size or be revoked, and stays valid for [`UPLOAD_TTL`], so it never points
/// at the key a file is served from: completing the upload copies the checked object to that
/// key, and anything written to the staging key afterwards is never served (and is deleted;
/// see [`Storage::delete_staging`]).
fn staging_key(key: &str) -> String {
    format!("staging/{key}")
}

/// How a stored file is read.
pub enum ReadTarget {
    /// Send the client to this URL (presigned or public).
    Redirect(String),
    /// Stream it from here; only the local provider.
    Stream,
}

/// The configured storage providers. Cheap to clone.
#[derive(Clone)]
pub struct Storage(Arc<Inner>);

struct Inner {
    providers: HashMap<Backend, Provider>,
    default: Backend,
    /// Base for upload URLs on this backend (local provider).
    backend_url: String,
    /// HMAC key for local upload URLs.
    upload_secret: Vec<u8>,
    max_file_size: u64,
    rate_limit: Duration,
}

impl Storage {
    /// Sets up the default provider, plus any other provider that is configured, so files
    /// uploaded before a provider switch still resolve. The local provider is only kept as
    /// a secondary one if its directory already exists.
    pub fn new(config: &StorageConfig, backend_url: &str, secret: &str) -> Result<Self, String> {
        let mut providers = HashMap::new();

        let local_path = std::path::Path::new(&config.local_path);
        if config.provider == StorageProvider::Local || local_path.is_dir() {
            std::fs::create_dir_all(local_path)
                .map_err(|e| format!("can't create STORAGE_LOCAL_PATH {local_path:?}: {e}"))?;
            let store = LocalFileSystem::new_with_prefix(local_path)
                .map_err(|e| format!("invalid STORAGE_LOCAL_PATH {local_path:?}: {e}"))?
                // Remove directories left empty by deletes.
                .with_automatic_cleanup(true);
            providers.insert(Backend::Local, Provider::Local(Arc::new(store)));
        }

        if let Some(s3) = &config.s3 {
            let mut builder = AmazonS3Builder::new()
                .with_bucket_name(&s3.bucket)
                .with_region(&s3.region)
                .with_access_key_id(&s3.access_key_id)
                .with_secret_access_key(&s3.secret_access_key);
            if let Some(endpoint) = &s3.endpoint {
                builder = builder
                    .with_endpoint(endpoint)
                    .with_allow_http(endpoint.starts_with("http://"));
            }
            let store = builder.build().map_err(|e| format!("invalid S3 configuration: {e}"))?;
            providers.insert(
                Backend::S3,
                Provider::S3 { store: Arc::new(store), public_url: s3.public_url.clone() },
            );
        }

        let default = match config.provider {
            StorageProvider::Local => Backend::Local,
            StorageProvider::S3 => Backend::S3,
        };

        // Domain-separated from the JWT signing key it's derived from.
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|e| format!("invalid JWT_SECRET: {e}"))?;
        mac.update(b"gravnet file upload");
        let upload_secret = mac.finalize().into_bytes().to_vec();

        Ok(Self(Arc::new(Inner {
            providers,
            default,
            backend_url: backend_url.to_owned(),
            upload_secret,
            max_file_size: config.max_file_size,
            rate_limit: Duration::from_secs(config.rate_limit_secs),
        })))
    }

    /// The provider new uploads go to.
    pub fn default_backend(&self) -> Backend {
        self.0.default
    }

    pub fn max_file_size(&self) -> u64 {
        self.0.max_file_size
    }

    pub fn rate_limit(&self) -> Duration {
        self.0.rate_limit
    }

    fn provider(&self, backend: Backend) -> Result<&Provider, ApiError> {
        self.0
            .providers
            .get(&backend)
            .ok_or_else(|| ApiError::Internal(format!("storage provider {backend} is not configured")))
    }

    /// Where the client should send the bytes of file `owner_id/file_id`, stored at `key`.
    pub async fn create_upload(
        &self,
        backend: Backend,
        owner_id: i64,
        file_id: &str,
        key: &str,
        content_type: &str,
    ) -> Result<UploadTarget, ApiError> {
        let expires_at = Utc::now() + UPLOAD_TTL;
        let headers = HashMap::from([("Content-Type", content_type.to_owned())]);
        let url = match self.provider(backend)? {
            Provider::Local(_) => {
                let expires = expires_at.timestamp();
                let signature = self.upload_signature(owner_id, file_id, expires);
                format!(
                    "{}/file/{owner_id}/{file_id}/content?expires={expires}&signature={signature}",
                    self.0.backend_url
                )
            }
            Provider::S3 { store, .. } => {
                // Content-Length can't be signed into a presigned URL, so the size is checked
                // when the upload is completed instead, on a staging copy (see `staging_key`).
                let value = HeaderValue::from_str(content_type)
                    .map_err(|_| ApiError::BadRequest("invalid content type".into()))?;
                let options = SignedUrlOptions::default()
                    .with_signed_header(HeaderName::from_static("content-type"), value);
                store
                    .signed_url_opts(Method::PUT, &Path::from(staging_key(key)), UPLOAD_TTL, &options)
                    .await
                    .map_err(storage_error)?
                    .to_string()
            }
        };
        Ok(UploadTarget { method: "PUT", url, headers, expires_at })
    }

    /// HMAC of a local upload URL's parameters, in hex.
    fn upload_signature(&self, owner_id: i64, file_id: &str, expires: i64) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.0.upload_secret).expect("any key length");
        mac.update(format!("{owner_id}/{file_id}/{expires}").as_bytes());
        hex(&mac.finalize().into_bytes())
    }

    /// Whether `signature` authorizes uploading to `owner_id/file_id` until `expires`, and
    /// that time hasn't passed.
    pub fn verify_upload(&self, owner_id: i64, file_id: &str, expires: i64, signature: &str) -> bool {
        if expires < Utc::now().timestamp() {
            return false;
        }
        let Some(signature) = crate::hex::decode(signature) else { return false };
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.0.upload_secret).expect("any key length");
        mac.update(format!("{owner_id}/{file_id}/{expires}").as_bytes());
        mac.verify_slice(&signature).is_ok()
    }

    /// Streams `body` to `key`, failing (and writing nothing) unless it is exactly `size`
    /// bytes. Only for providers that accept uploads through this backend (local).
    pub async fn write<S, E>(&self, backend: Backend, key: &str, size: u64, mut body: S) -> Result<(), ApiError>
    where
        S: Stream<Item = Result<Bytes, E>> + Unpin,
        E: fmt::Display,
    {
        let store = self.provider(backend)?.store();
        let upload = store.put_multipart(&Path::from(key)).await.map_err(storage_error)?;
        let mut writer = WriteMultipart::new(upload);
        let mut written = 0u64;
        while let Some(chunk) = body.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(e) => {
                    writer.abort().await.ok();
                    return Err(ApiError::BadRequest(format!("upload interrupted: {e}")));
                }
            };
            written += chunk.len() as u64;
            if written > size {
                writer.abort().await.ok();
                return Err(ApiError::BadRequest(format!("upload is larger than the declared {size} bytes")));
            }
            writer.write(&chunk);
        }
        if written != size {
            writer.abort().await.ok();
            return Err(ApiError::BadRequest(format!(
                "upload is {written} bytes but {size} were declared"
            )));
        }
        writer.finish().await.map_err(storage_error)?;
        Ok(())
    }

    /// The stored object's size and ETag, or `None` if nothing is stored at `key`.
    async fn head(store: &dyn ObjectStore, key: &str) -> Result<Option<(u64, Option<String>)>, ApiError> {
        match store.head(&Path::from(key)).await {
            Ok(meta) => Ok(Some((meta.size, meta.e_tag))),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(storage_error(e)),
        }
    }

    /// Checks that the bytes uploaded for `key` are exactly `size` bytes and puts them in
    /// place: the local provider wrote them to `key` already, while an S3 upload is copied
    /// there from its staging key (see [`staging_key`]), which is then deleted. An upload of
    /// the wrong size is deleted.
    pub async fn finish_upload(&self, backend: Backend, key: &str, size: u64) -> Result<Finish, ApiError> {
        let provider = self.provider(backend)?;
        let store = provider.store();
        if let Provider::Local(_) = provider {
            return Ok(match Self::head(store, key).await? {
                None => Finish::Missing,
                Some((found, _)) if found != size => {
                    self.delete(backend, key).await?;
                    Finish::WrongSize(found)
                }
                Some((_, e_tag)) => Finish::Done(e_tag),
            });
        }

        let staging = staging_key(key);
        match Self::head(store, &staging).await? {
            None => return Ok(Finish::Missing),
            // Rejected before copying, so an oversized upload isn't copied.
            Some((found, _)) if found != size => {
                self.delete_staging(backend, key).await?;
                return Ok(Finish::WrongSize(found));
            }
            Some(_) => {}
        }
        match store.copy(&Path::from(staging.as_str()), &Path::from(key)).await {
            Ok(()) => {}
            Err(object_store::Error::NotFound { .. }) => return Ok(Finish::Missing),
            Err(e) => return Err(storage_error(e)),
        }
        self.delete_staging(backend, key).await?;
        // Checked again on the copy: the staging object could have been replaced between
        // the check above and the copy.
        match Self::head(store, key).await? {
            None => Ok(Finish::Missing),
            Some((found, _)) if found != size => {
                self.delete(backend, key).await?;
                Ok(Finish::WrongSize(found))
            }
            Some((_, e_tag)) => Ok(Finish::Done(e_tag)),
        }
    }

    /// Deletes what an upload to `key` may have left at its staging key (S3 only; a no-op for
    /// the local provider). Writes through the presigned URL remain possible until it
    /// expires, so this is also run once it has (see `main::sweep_staging`).
    pub async fn delete_staging(&self, backend: Backend, key: &str) -> Result<(), ApiError> {
        match self.provider(backend)? {
            Provider::Local(_) => Ok(()),
            Provider::S3 { .. } => self.delete(backend, &staging_key(key)).await,
        }
    }

    /// Deletes a file's object and anything left at its staging key.
    pub async fn delete_file(&self, backend: Backend, key: &str) -> Result<(), ApiError> {
        self.delete(backend, key).await?;
        self.delete_staging(backend, key).await
    }

    /// How to serve `key` to a client. A `private` file is never sent to the bucket's public
    /// URL, which anyone could reuse, but to a short-lived presigned one.
    pub async fn read_target(&self, backend: Backend, key: &str, private: bool) -> Result<ReadTarget, ApiError> {
        match self.provider(backend)? {
            Provider::Local(_) => Ok(ReadTarget::Stream),
            Provider::S3 { public_url: Some(base), .. } if !private => {
                Ok(ReadTarget::Redirect(format!("{base}/{key}")))
            }
            Provider::S3 { store, .. } => {
                let url = store
                    .signed_url(Method::GET, &Path::from(key), READ_TTL)
                    .await
                    .map_err(storage_error)?;
                Ok(ReadTarget::Redirect(url.to_string()))
            }
        }
    }

    /// The object's bytes, or `None` if it's missing.
    pub async fn get(
        &self,
        backend: Backend,
        key: &str,
    ) -> Result<Option<impl Stream<Item = Result<Bytes, object_store::Error>> + use<>>, ApiError> {
        match self.provider(backend)?.store().get(&Path::from(key)).await {
            Ok(result) => Ok(Some(result.into_stream())),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(e) => Err(storage_error(e)),
        }
    }

    /// Deletes `key`. Deleting a missing object succeeds.
    pub async fn delete(&self, backend: Backend, key: &str) -> Result<(), ApiError> {
        match self.provider(backend)?.store().delete(&Path::from(key)).await {
            Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(e) => Err(storage_error(e)),
        }
    }
}

fn storage_error(e: object_store::Error) -> ApiError {
    ApiError::Internal(format!("storage: {e}"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storage(dir: &std::path::Path) -> Storage {
        let config = StorageConfig {
            provider: StorageProvider::Local,
            local_path: dir.to_string_lossy().into_owned(),
            s3: None,
            max_file_size: 16,
            rate_limit_secs: 3,
        };
        Storage::new(&config, "http://localhost:8080", "0123456789abcdef0123456789abcdef").unwrap()
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("gravnet-storage-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn body(chunks: &[&'static [u8]]) -> impl Stream<Item = Result<Bytes, std::convert::Infallible>> + Unpin {
        futures_util::stream::iter(chunks.iter().map(|c| Ok(Bytes::from_static(c))).collect::<Vec<_>>())
    }

    #[test]
    fn upload_signatures() {
        let dir = temp_dir("sig");
        let storage = storage(&dir);
        let expires = Utc::now().timestamp() + 60;
        let sig = storage.upload_signature(1, "abc", expires);
        assert!(storage.verify_upload(1, "abc", expires, &sig));
        assert!(!storage.verify_upload(2, "abc", expires, &sig));
        assert!(!storage.verify_upload(1, "abd", expires, &sig));
        assert!(!storage.verify_upload(1, "abc", expires + 1, &sig));
        assert!(!storage.verify_upload(1, "abc", expires, "zz"));
        let past = Utc::now().timestamp() - 1;
        let sig = storage.upload_signature(1, "abc", past);
        assert!(!storage.verify_upload(1, "abc", past, &sig));
        std::fs::remove_dir_all(dir).ok();
    }

    #[actix_web::test]
    async fn local_round_trip() {
        let dir = temp_dir("rt");
        let storage = storage(&dir);
        let key = "files/1/abc";

        let err = storage.write(Backend::Local, key, 3, body(&[b"ab", b"cd"])).await;
        assert!(err.is_err(), "larger than declared");
        let err = storage.write(Backend::Local, key, 5, body(&[b"ab", b"cd"])).await;
        assert!(err.is_err(), "smaller than declared");

        assert!(matches!(storage.finish_upload(Backend::Local, key, 4).await.unwrap(), Finish::Missing));

        storage.write(Backend::Local, key, 4, body(&[b"ab", b"cd"])).await.unwrap();
        assert!(matches!(storage.finish_upload(Backend::Local, key, 4).await.unwrap(), Finish::Done(_)));
        let bytes: Vec<Bytes> = storage
            .get(Backend::Local, key)
            .await
            .unwrap()
            .unwrap()
            .map(|c| c.unwrap())
            .collect()
            .await;
        assert_eq!(bytes.concat(), b"abcd");

        storage.delete(Backend::Local, key).await.unwrap();
        storage.delete(Backend::Local, key).await.unwrap();
        assert!(storage.get(Backend::Local, key).await.unwrap().is_none());
        std::fs::remove_dir_all(dir).ok();
    }
}
