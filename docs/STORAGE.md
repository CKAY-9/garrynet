# Storage

Part of [GarryNet Design](README.md).

Files are stored locally or in an S3-compatible bucket (AWS S3, Cloudflare R2, MinIO, ...), depending on backend configuration. Every provider sits behind one interface (`backend/src/storage.rs`, built on the [`object_store`](https://docs.rs/object_store) crate), so routes never care where a file lives.

## Configuration

Environment variables (see `backend/.env.example`):

- `STORAGE_PROVIDER`: where new uploads go, `local` (default) or `s3`
- `STORAGE_LOCAL_PATH`: root directory of the local provider (default = `./storage`)
- `STORAGE_MAX_FILE_SIZE`: in bytes (default = 2097152, 2MB)
- `STORAGE_RATE_LIMIT`: average seconds between uploads by the same user (default = 3). Up to 10 uploads can be made back to back (enough for one message's attachments), so the limit is 10 uploads per 10 × this many seconds
- `S3_BUCKET`, `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`: enable the S3 provider
- `S3_REGION`: default `us-east-1`; use `auto` for R2
- `S3_ENDPOINT`: for non-AWS stores, e.g. `https://<account>.r2.cloudflarestorage.com`
- `S3_PUBLIC_URL`: optional public base URL of the bucket (custom domain or CDN); reads redirect there instead of to presigned URLs (except for support and private message attachments, which are private; see [GET /file](#get-fileowner_idfile_id))

Each file records the provider it was uploaded to, so switching `STORAGE_PROVIDER` only affects new uploads. Old files keep resolving as long as their provider stays configured. The local provider stays available while its directory exists.

## Testing with a local S3 (Garage)

To test the S3 provider without an AWS account, run [Garage](https://garagehq.deuxfleurs.fr/), a self-hosted S3-compatible store, in Docker. From the repo root, using [garage.toml](../garage.toml):

```sh
docker run -d --name garage-container \
  -p 127.0.0.1:3900:3900 -p 127.0.0.1:3901:3901 -p 127.0.0.1:3902:3902 -p 127.0.0.1:3903:3903 \
  -v "$(pwd)/garage.toml:/etc/garage.toml" \
  -v garage-meta:/var/lib/garage/meta -v garage-data:/var/lib/garage/data \
  -e GARAGE_DEFAULT_ACCESS_KEY=GK0123456789abcdef01234567 \
  -e GARAGE_DEFAULT_SECRET_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef \
  -e GARAGE_DEFAULT_BUCKET=garrynet \
  dxflrs/garage:v2.5.0 /garage server --single-node --default-bucket
```

`--single-node` sets up the cluster layout itself and `--default-bucket` creates the bucket and key from the `GARAGE_DEFAULT_*` variables. Garage requires the access key to be `GK` plus 24 hex characters and the secret to be 64 hex characters. These credentials are for local testing only.

Then set these in `backend/.env`:

```
STORAGE_PROVIDER=s3
S3_BUCKET=garrynet
S3_ENDPOINT=http://localhost:3900
S3_REGION=garage
S3_ACCESS_KEY_ID=GK0123456789abcdef01234567
S3_SECRET_ACCESS_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
```

Manage the container with `docker stop garage-container` and `docker start garage-container`. Data persists in the `garage-meta` and `garage-data` volumes; remove them with `docker rm garage-container && docker volume rm garage-meta garage-data` to start fresh. Browser uploads need a CORS rule on the bucket allowing `PUT` from `FRONTEND_URL`. Garage sets it through the S3 `PutBucketCors` call, not `garage.toml`, and it persists in the volumes. With a `cors.xml` like the one below, apply it with curl (no AWS CLI needed):

```xml
<CORSConfiguration><CORSRule>
  <AllowedOrigin>http://localhost:3000</AllowedOrigin>
  <AllowedMethod>PUT</AllowedMethod><AllowedMethod>GET</AllowedMethod><AllowedMethod>HEAD</AllowedMethod>
  <AllowedHeader>*</AllowedHeader>
  <ExposeHeader>ETag</ExposeHeader>
  <MaxAgeSeconds>3000</MaxAgeSeconds>
</CORSRule></CORSConfiguration>
```

```sh
curl -X PUT --aws-sigv4 "aws:amz:garage:s3" \
  --user "GK0123456789abcdef01234567:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" \
  -H "Content-Type: application/xml" --data-binary @cors.xml "http://localhost:3900/garrynet?cors"
```

A `NetworkError when attempting to fetch resource` in the browser after `POST /file/upload` returns 201 usually means this rule is missing.

## File

```
owner_id: number // user ID, or -1 for community files
id: string // 32 random hex characters, unique within the owner's folder
backend: "local" | "s3"
key: string // object key in that provider: files/{owner_id}/{id}
name: string // sanitized client file name, only used as the download name
content_type: string
size: number // bytes, declared at upload and verified after
e_tag?: string // provider's ETag when verified
status: PENDING | READY
uploaded_by?: number // user ID
created_at: utc timestamp
completed_at?: utc timestamp
forum_post_id?: number // the forum post it's attached to
support_message_id?: number // the support message it's attached to
private_message_id?: number // the private message it's attached to
```

The database stores keys, not URLs. URLs are built when a file is read, so files can move between providers without rewriting rows. Keys are generated by the backend; client file names never reach them.

Allowed content types: `image/png`, `image/jpeg`, `image/gif`, `image/webp`, `image/avif`, `audio/mpeg`, `audio/ogg`, `audio/wav`, `video/mp4`, `video/webm`, `application/pdf`, `text/plain`. Nothing a browser would run as a document (no HTML or SVG).

Clients see a file as `{ owner_id, id, url, name, content_type, size, status, uploaded_by, created_at, completed_at }`, where `url` is `{BACKEND_URL}/file/{owner_id}/{id}`.

## Where files are used

- Attachments: up to 5 files per forum post ([FORUM.md](FORUM.md)), 10 per support message ([SUPPORT.md](SUPPORT.md)) and 3 per private message ([PRIVATE_MESSAGES.md](PRIVATE_MESSAGES.md)). These are user files. A file can be attached to one thing only, by its owner, when the post or message is written; it's deleted when its forum post or thread is deleted. A file attached to a private message is kept when the message expires or a user deletes the chat.
- Image links in the admin pages (community logo, splash and key figure pictures, forum section icons, server icons and splashes, store item images): each link input has an Upload button that stores the image as a community file and fills the input with its URL. A link can still be pasted instead.
- User avatars come from Steam and can't be uploaded.

Files that are uploaded but never attached or used (for example, the page was closed before posting) are not cleaned up yet.

## Uploading

Uploading takes three steps, the same for every provider:

1. `POST /file/upload` records a pending file and returns where to send its bytes.
2. The client sends the bytes there: `PUT` with the returned headers.
3. `POST /file/{owner_id}/{id}/complete` verifies the bytes and marks the file ready.

The frontend does steps 1 and 3 from server actions and step 2 from the browser (`frontend/src/components/Upload`).

With the S3 provider, step 2 goes straight to the bucket through a presigned URL, so file bytes never pass through the backend. The bucket needs a CORS rule allowing `PUT` with a `Content-Type` header from `FRONTEND_URL`. With the local provider, step 2 goes to `PUT /file/{owner_id}/{id}/content`, which streams to disk and completes the file itself (step 3 is then a no-op).

Pending uploads still pending after an hour are deleted, along with any bytes that arrived.

### POST /file/upload

Requires a login, but not Site Access: banned users can still use support, which includes attaching files to their tickets. Where a file can then be attached is checked when it's attached (forum posts need Site Access). Community files need Admin Access. Body:

```
name: string // client file name
content_type: string // one of the allowed types
size: number // bytes, 1 to STORAGE_MAX_FILE_SIZE
community?: boolean // store as a community file (/file/-1/{id}); requires Admin Access
```

Returns 201 with `{ file, upload: { method: "PUT", url, headers, expires_at } }`. The upload URL is valid for 15 minutes. Returns 429 with `Retry-After` once the user is over the upload rate limit.

### PUT /file/{owner_id}/{id}/content

Local provider only. Authorized by the signature in the URL from `/file/upload` (an HMAC derived from `JWT_SECRET`), not by login. `Content-Type` must match the declared type, and the body must be exactly the declared size. Answers CORS requests from `FRONTEND_URL` (including the `OPTIONS` preflight), since browsers upload to it directly. Returns the ready file. A second upload to a ready file returns 409.

### POST /file/{owner_id}/{id}/complete

Only the uploader can call this. Checks that the stored object exists and has the declared size, then marks the file ready and returns it. A presigned S3 URL can bind the content type but not the size, so an object of the wrong size is deleted here and the client must upload again. Completing a ready file just returns it.

## GET /file/{owner_id}/{id}

Fetch the requested file. Public, since file IDs are unguessable, except for support and private message attachments (below). Returns 404 until the file is ready.

- Local: streamed by the backend with `Content-Type`, `Content-Disposition: inline`, an `ETag` (`If-None-Match` gives 304), `Cache-Control: public, max-age=31536000, immutable`, `X-Content-Type-Options: nosniff` and a sandboxing `Content-Security-Policy`.
- S3: a 302 redirect to `{S3_PUBLIC_URL}/{key}` if set, otherwise to a presigned URL valid for an hour.

**Support attachments** (files attached to a support message) are private: they're only served to the ticket's creator and users with Support Team, and an internal note's files only to Support Team ([SUPPORT.md](SUPPORT.md)). The requester is identified like any route, by the auth cookie (which the browser sends with `<img>` and link requests, as the frontend and backend share a site) or `Authorization: Bearer`. Everyone else gets 404, not 403, so file IDs can't be probed. Before a file is attached it's still public by its unguessable URL, which only its uploader knows.

**Private message attachments** (files attached to a private message) are just as private: they're only served to the message's author, its recipient and users with Admin Access and User Management ([PRIVATE_MESSAGES.md](PRIVATE_MESSAGES.md)), whether or not the chat was deleted or the message expired. Everyone else gets 404, as above.

Both kinds are served differently from public files, so access is checked on every view:

- Local: `Cache-Control: private, no-cache`, so only the viewer's browser caches it and revalidates it (a 304) each time.
- S3: always a presigned URL, even with `S3_PUBLIC_URL`, and the redirect is `Cache-Control: private, no-store`. With `S3_PUBLIC_URL` the bucket itself is public, though: anyone who learns an object's key (it's in the presigned URL) can read it there directly. If support or private message attachments must stay private, don't make the bucket public.

## DELETE /file/{owner_id}/{id}

Deletes the file and its bytes. Returns 204.

- Community files: Admin Access and Community Management, even for their uploader, since they may be in use as a logo, icon or splash.
- User files: the uploader, until the file is attached to a forum post, support message or private message (400 after that: it's deleted with what it's attached to, so evidence in a ticket can't be pulled once staff have seen it); and users with Admin Access and User Management at any time.
