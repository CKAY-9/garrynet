# Architecture

Part of [GarryNet Design](README.md).

GarryNet has three parts: a Rust backend that owns all data, a Next.js frontend for the website and loading screen, and a Garry's Mod addon on each game server.

```
                 ┌──────────────── Steam OpenID login, file uploads ────────────────┐
                 │                                                                  ▼
 Browser ──► Frontend (Next.js) ── server-side requests, user JWT as bearer ──► Backend (Actix Web) ──► PostgreSQL
                                                                                 ▲   │
 Garry's Mod server ── heartbeat, store fulfillments, server JWT ────────────────┘   ├──► Stripe (checkout, webhooks)
 (GarryNet addon)                                                                    ├──► Steam Web API (names, avatars)
                                                                                     └──► Local disk or S3 (files)
```

## Backend

`backend/`: Actix Web (Rust) with PostgreSQL through sqlx. The backend is the only part that touches the database. Migrations in `backend/migrations/` are embedded in the binary and applied on startup; never edit one that has been released, since sqlx refuses to start when an applied migration's checksum changes.

- `src/routes/`: one module per feature, matching the docs in this folder (forum, store, support, ...). `gnet.rs` holds the routes the addon calls.
- `src/models/`: database models and queries.
- `src/auth/`: Steam OpenID and JWTs (see [AUTH.md](AUTH.md)).
- `src/stripe.rs`: Stripe Checkout and webhook handling (see [STORE.md](STORE.md)).
- `src/storage.rs`: file storage on local disk or an S3-compatible bucket through `object_store` (see [STORAGE.md](STORAGE.md)).
- `src/notify.rs`: creating notifications (see [NOTIFICATIONS.md](NOTIFICATIONS.md)).
- `src/shell.rs`: the admin command shell, available interactively or as `backend <command>` (see [PERMISSIONS.md](PERMISSIONS.md)).

Background tasks run inside the backend process (`src/main.rs`): auto-closing support tickets left waiting on the user, settling stale store purchases with Stripe, deleting abandoned uploads and files never attached to anything, and removing old notifications.

## Frontend

`frontend/`: Next.js (App Router) with TypeScript and SCSS modules. See [frontend/README.md](../frontend/README.md).

The frontend has no database. Pages and server actions call the backend from the Next.js server, authenticating as the user with the httpOnly cookie the backend set at login. The browser talks to the backend directly in only a few cases: Steam login and logout redirects, and file uploads (which go to the backend, or straight to the S3 bucket through a presigned URL).

`/loading-screen` is rendered by the frontend but opened by the Garry's Mod client through `sv_loadingurl`, so it works without a login (see [SERVERS.md](SERVERS.md#loading-screen)).

## Addon

`addon/`: a standard GLua server addon (see [ADDON.md](ADDON.md)). It links to the backend once with an access key and then authenticates with a rolling server JWT. Every minute it sends a heartbeat (map, gamemode, players, stats) and fetches store fulfillments to deliver. All traffic is addon → backend; the backend never connects to game servers.

## Authentication

- **Users:** Steam OpenID through the backend, which sets a 24-hour JWT in an httpOnly cookie (`garrynet_token`).
- **Servers:** a one-time access key exchanged for a 30-day rolling JWT, revoked by bumping the server's `token_version`.
- **Uploads (local provider):** signed URLs, HMAC'd with a key derived from `JWT_SECRET`.
- **Stripe webhooks:** verified with `STRIPE_WEBHOOK_SECRET`.

See [AUTH.md](AUTH.md) for details.
