<div align="center">
    <img src="./assets/logo.svg" alt="GravNet logo" width="250" height="250" />
    <br><br>
    <h1>GravNet</h1>
    <p>An all-in-one, self-hosted website for Garry's Mod communities: forums, a store, support tickets, loading screens and live server sync, all in one deployment.</p>
</div>

<p align="center">
    <img src="./assets/showcase/home.png" alt="Community homepage" width="672" height="400" />
</p>

> [!NOTE]
> GravNet is under active development. The features below work, but expect breaking changes between versions.

## Features

### Community Homepage

One deployment per community. A homepage with your name, logo, splash image, external links, key figures and your servers (with `steam://` connect links). Everything is editable from the admin dashboard.

### Loading Screen

A loading screen for your Garry's Mod servers, in two styles (Aero and Sleek). It greets each player by name and avatar, shows their first join and last visit on that server, and follows Garry's Mod's download progress.

<p align="center">
    <img src="./assets/showcase/loading-screen.png" alt="Loading screen" width="672" height="400" />
</p>

### Forums

Sections, categories and threads with Markdown posts, attachments, likes/dislikes and user reputation. Moderators can lock, sticky, edit and delete.

<p align="center">
    <img src="./assets/showcase/forums.png" alt="Forums" width="672" height="400" />
</p>

### Support

Support tickets with categories, internal staff notes and attachments, plus an FAQ. Users banned from the site can still open tickets.

### Store

Sell in-game rewards per server: usergroups (permanent or timed), weapons and DarkRP money. Payments go through Stripe Checkout, with discounts, stock limits, receipts and refunds. The addon delivers purchases in-game automatically, and refunds take usergroups and weapons back.

<p align="center">
    <img src="./assets/showcase/store.png" alt="Store" width="672" height="400" />
</p>

### Users and Permissions

Steam login, profiles with bios and their threads and posts, private profiles, online status and user search. Usergroups have permission bits and custom colors, icons and effects. Staff can ban users (support only) or suspend them (no posting) from their profile, permanently or for a set time, and optionally keep banned users off your game servers.

<p align="center">
    <img src="./assets/showcase/admin-usergroups.png" alt="Admin usergroups" width="672" height="400" />
</p>

### Servers and Players

Register servers on the website and link them with a one-time access key. The site shows each server's status, gamemode and map. The addon syncs per-server player records every minute: playtime, last seen and gamemode stats such as DarkRP money and job. It can also refuse players banned on the website.

### Legal Pages

Terms of service, a privacy policy and a refund policy, linked from every page, the login button and the store. GravNet fills them with templates based on what it actually stores, and you edit them from the admin dashboard. The templates are a starting point, not legal advice.

### Notifications

In-site notifications for ticket replies, purchases and deliveries, forum replies, private messages, usergroup changes, and bans and suspensions.

### Private Messages

Users with the `User Messages` permission can message each other with Markdown and up to 3 attachments. Messages can be set to disappear after 24 hours, and either user can delete a chat from their own side. Messages aren't private from staff, who can read them from the admin dashboard.

## How It Fits Together

```
 Browser ──► Frontend (Next.js) ──► Backend (Rust, Actix Web) ──► PostgreSQL
    │                                   ▲   ▲   │
    └── Steam login, uploads ───────────┘   │   └──► Stripe, S3 (optional)
                                            │
 Garry's Mod server (GravNet addon) ───────┘  heartbeat, store delivery
```

- **Backend** (`backend/`): the API. Steam login, permissions, forums, store, Stripe webhooks, file storage, and the routes the addon calls. Database migrations are applied on startup.
- **Frontend** (`frontend/`): the website and the loading screen. It talks to the backend server-side, and the browser talks to the backend directly for login and uploads.
- **Addon** (`addon/`): a Garry's Mod server addon. It links the server, reports players every minute and delivers store purchases.

Both the browser and your Garry's Mod servers must be able to reach the backend, so it needs a public URL (e.g. `https://api.example.com`).

Everything outside localhost runs on HTTPS: the backend (in release builds), the frontend (in production builds) and the addon refuse plain-HTTP URLs to other machines, since login cookies, tokens and server keys travel over them. See [HTTPS](#https).

## Requirements

- A [Steam Web API key](https://steamcommunity.com/dev/apikey)
- **Docker** with Docker Compose, **or** PostgreSQL 17, Rust 1.85+ and Node.js 22
- A domain for the site and one for the backend (e.g. `example.com` and `api.example.com`). With Docker, Caddy serves both with HTTPS; a manual setup needs its own reverse proxy
- Optional: a [Stripe](https://stripe.com) account, to take payments in the store
- Optional: an S3-compatible bucket (AWS S3, Cloudflare R2, MinIO) for uploads; local disk is the default

## Deployment

### Docker (recommended)

1. **Clone** the repository onto your web server, and point DNS records for both domains at it with ports 80 and 443 open:
    ```sh
    git clone https://github.com/CKAY-9/gravnet.git
    cd gravnet
    ```
2. **Configure the backend.** Copy `backend/.env.example` to `backend/.env` and set:
    - `BACKEND_URL`: the backend's public URL, e.g. `https://api.example.com`. Steam login returns here.
    - `FRONTEND_URL`: the website's public URL, e.g. `https://example.com`.
    - `JWT_SECRET`: a long random string, e.g. from `openssl rand -base64 48`.
    - `STEAM_API_KEY`: your Steam Web API key.
    - `COOKIE_DOMAIN`: needed when the site and backend are on different subdomains. For `example.com` and `api.example.com`, set it to `example.com`.

    `DATABASE_URL`, `DATABASE_PASSWORD_FILE` and `BIND_ADDR` are set by Compose, so you can leave them alone.
3. **Configure the frontend.** Copy `frontend/.env.example` to `frontend/.env` and set `NEXT_PUBLIC_API_URL` to the backend's public URL (the same as `BACKEND_URL`). `API_URL` is set by Compose.
4. **Set the domains.** Copy `.env.example` to `.env` in the repository root and set:
    ```sh
    SITE_DOMAIN=example.com
    API_DOMAIN=api.example.com
    ```
    The database password is generated on first start: a random 256-bit one, kept in the `db-secrets` volume, which only the database and backend mount. To choose your own, set `POSTGRES_PASSWORD` here before the first start. You can also set `POSTGRES_USER`, `POSTGRES_DB` and the rate limit (see [Reverse proxy](#reverse-proxy)) here.
5. **Start it:**
    ```sh
    docker compose up -d --build
    ```
    Caddy gets HTTPS certificates for both domains on first start. Only Caddy publishes ports (80 and 443); the frontend and backend are reached through it.
6. **Make yourself an admin.** Log in on the website with Steam once, then:
    ```sh
    docker compose exec backend backend usergroup add <your SteamID64> admin
    ```
    Reload the page and the Admin section appears in the navigation.

`NEXT_PUBLIC_API_URL` is baked into the frontend at build time, so re-run with `--build` after changing it. Uploaded files are kept in the `uploads` volume, the database in `db-data` (and its password in `db-secrets`), and Caddy's certificates in `caddy-data`. To see the password, e.g. to connect with `psql`: `docker compose exec db cat /run/secrets/gravnet/postgres_password`.

#### Reverse proxy

Caddy ([Caddyfile](./Caddyfile)) is built with the [rate limit plugin](https://github.com/mholt/caddy-ratelimit) (`caddy/Dockerfile`). On both domains it:

- **Sets security headers:** `Strict-Transport-Security: max-age=31536000` (HSTS: browsers that have visited use HTTPS only, for a year; not `includeSubDomains`, since other services may share the parent domain over HTTP), `Content-Security-Policy: frame-ancestors 'self'`, `X-Frame-Options: SAMEORIGIN`, `X-Content-Type-Options: nosniff` and `Referrer-Policy: strict-origin-when-cross-origin`, unless the frontend or backend already set them, and removes the `Server` header.
- **Rate limits each client IP:** at most `RATE_LIMIT_EVENTS` (default 600) requests per `RATE_LIMIT_WINDOW` (default `1m`), counted separately for the site and the API. Next.js build assets (`/_next/static/`) aren't counted. Game servers' `/gnet/` requests have their own limit, `RATE_LIMIT_GNET_EVENTS` (default 1200), since servers on one host share an IP. Requests over the limit get `429 Too Many Requests` with `Retry-After`. Set these in the root `.env` and run `docker compose up -d` to apply them.

If another proxy or CDN (e.g. Cloudflare) sits in front of Caddy, add its IP ranges as [`trusted_proxies`](https://caddyserver.com/docs/caddyfile/options#trusted-proxies) in the Caddyfile's global options. Otherwise every visitor counts against the proxy's IP.

### Demo

To run a public demo, with Stripe in test mode and everything non-admins create cleared daily, add `docker-compose.demo.yml`. See [docs/DEMO.md](./docs/DEMO.md#deploying).

```sh
docker compose -f docker-compose.yml -f docker-compose.demo.yml up -d --build
```

### Manual

The backend and frontend are separate processes; start the backend first.

**Backend**

1. Create a PostgreSQL database.
2. Copy `backend/.env.example` to `backend/.env` and set `DATABASE_URL`, plus the variables from step 2 of the Docker setup.
3. In `backend/`, run `cargo run --release`. Migrations are applied on startup.

**Frontend**

1. Copy `frontend/.env.example` to `frontend/.env`. Set `NEXT_PUBLIC_API_URL` to the backend's public URL (`https://`) and `API_URL` to an address the frontend server can reach it on (e.g. `http://localhost:8080`).
2. In `frontend/`, run `npm ci && npm run build && npm start`.

Then make yourself an admin. Log in once, and in `backend/` run `cargo run --release -- usergroup add <your SteamID64> admin`.

A manual setup needs its own reverse proxy terminating HTTPS in front of both (see [HTTPS](#https)).

### HTTPS

Outside localhost, every URL GravNet is configured with must be `https://`. Plain HTTP is allowed only for `localhost`, `*.localhost` and loopback addresses (`127.x.x.x`, `[::1]`), and, for URLs only the server itself uses, single-label names on a private network such as Compose's `http://backend:8080`. Otherwise:

- **Backend** (release builds, i.e. `cargo run --release` and the Docker image): refuses to start if `BACKEND_URL`, `FRONTEND_URL` or `S3_PUBLIC_URL` is plain HTTP, if `S3_ENDPOINT` is plain HTTP to a host that isn't local or single-label, or if `COOKIE_SECURE=false`. Development builds (`cargo run`) only warn. When `BACKEND_URL` is HTTPS it sends `Strict-Transport-Security: max-age=31536000` itself, for setups without Caddy.
- **Frontend** (production builds): `next build` fails if `NEXT_PUBLIC_API_URL` is plain HTTP, and the server refuses requests (500, with the reason in its log) if `API_URL` or `NEXT_PUBLIC_API_URL` is. It sends `Strict-Transport-Security` in production; browsers ignore it over plain HTTP, so `npm start` on localhost still works. `npm run dev` allows plain HTTP.
- **Addon:** sends nothing to a plain-HTTP `gravnet_api_url` on another machine, and says why in the console, `gravnet_status` and the `!gnet` menu. For development against a backend elsewhere on your network, `gravnet_allow_http 1` allows it, with a warning.
- **Caddy** (Docker) serves both domains over HTTPS, redirects plain HTTP and sends HSTS (see [Reverse proxy](#reverse-proxy)).

### Updating

```sh
git pull
docker compose up -d --build
```

Postgres only reads its password when the database is first created. A database created before passwords were generated, without `POSTGRES_PASSWORD` set, still has the old default password, `gravnet`, and the backend can't connect after updating. Switch it to the generated password once:

```sh
docker compose up -d db
docker compose exec db sh -c 'echo "ALTER ROLE CURRENT_USER PASSWORD :'"'"'pw'"'"';" | psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -v pw="$(cat /run/secrets/gravnet/postgres_password)"'
docker compose up -d
```

(Inside the container, `psql` connects over the local socket, which needs no password.) If you had set `POSTGRES_PASSWORD`, nothing changes: it's still used.

Database migrations run automatically when the backend starts. Back up the `db-data` volume before updating.

## Connecting a Garry's Mod Server

1. **Install the addon.** Copy the `addon/` folder to your server as `garrysmod/addons/gravnet`.
2. **Point it at the backend.** Add this to `garrysmod/cfg/server.cfg`, using the backend's public URL (`BACKEND_URL`):
    ```
    gravnet_api_url "https://api.example.com"
    ```
    Garry's Mod blocks HTTP requests to local and private addresses. If the backend runs on the same machine or network (common when testing), start the server with `-allowlocalhttp`.
3. **Register the server.** On the website, go to **Admin → Servers → Register** and copy the access key. It is only shown once.
4. **Link it.** Restart the Garry's Mod server, join as a superadmin, type `!gnet` in chat and paste the key. (From the server console: `gravnet_link <key>`.) Superadmins get a chat message once the server is linked, and the website shows it as **Online** within a minute.
5. **Set up the loading screen** (optional). The server's admin page shows its `sv_loadingurl` line. Add it to `server.cfg`:
    ```
    sv_loadingurl "https://example.com/loading-screen?server=<SERVER_ID>&steamid=%s&mapname=%m"
    ```

If the server and website fall out of sync (for example, the server was unlinked on the website), regenerate the access key and link again. Store usergroups are granted through ULX when it's installed. See [addon/README.md](./addon/README.md) for console commands and store delivery details.

## Store Payments (Stripe)

The store can be browsed and managed without Stripe, but nothing can be bought until both keys are set in `backend/.env`:

1. `STRIPE_SECRET_KEY`: from the [Stripe dashboard](https://dashboard.stripe.com/apikeys) (`sk_live_...`, or `sk_test_...` for testing).
2. Create a webhook endpoint at `<BACKEND_URL>/stripe/webhook` subscribed to these events, and put its signing secret in `STRIPE_WEBHOOK_SECRET`:
    - `checkout.session.completed`
    - `checkout.session.async_payment_succeeded`
    - `checkout.session.async_payment_failed`
    - `checkout.session.expired`
    - `charge.refunded`
    - `charge.refund.updated`
    - `charge.dispute.created`
    - `charge.dispute.closed`

Stripe Tax is optional (`STRIPE_TAX`). See [docs/STORE.md](./docs/STORE.md) for stores, items, rewards and refunds.

## Administration

Most management happens in the website's **Admin** section, which you can reach once you're in a usergroup with Admin Access. The backend also has a small command shell for when you're locked out or bootstrapping:

```sh
backend usergroup list
backend usergroup add <USER_ID or SteamID64> <GROUP>
backend usergroup remove <USER_ID or SteamID64> <GROUP>
backend user show <USER_ID or SteamID64>
backend user unban <USER_ID or SteamID64>
backend user unsuspend <USER_ID or SteamID64>
backend demo reset          # demo deployments only: clear what non-admins created
backend help
```

With Docker, prefix these with `docker compose exec backend`. You can also type the commands, without the `backend` prefix, straight into a running backend's terminal:

- **Docker:** run `docker compose attach backend` and type commands such as `usergroup list`. Detach with **Ctrl+P, Ctrl+Q**. Ctrl+C stops the backend, which restarts automatically.
- **Manual:** type them into the terminal running `cargo run`.

Command output and the server's logs appear in the same terminal. See [docs/PERMISSIONS.md](./docs/PERMISSIONS.md) for the permission bits.

## Development

Run the backend and frontend in development mode:

```sh
# Terminal 1: needs a local PostgreSQL and backend/.env
cd backend && cargo run

# Terminal 2
cd frontend && npm install && npm run dev
```

- **Stripe:** forward webhooks to your local backend with `stripe listen --forward-to localhost:8080/stripe/webhook`, and use the `whsec_...` it prints as `STRIPE_WEBHOOK_SECRET`.
- **Garry's Mod:** run a local server with `-allowlocalhttp` and symlink `addon/` into its `addons` folder.
- **S3:** [docs/STORAGE.md](./docs/STORAGE.md#testing-with-a-local-s3-garage) shows how to test the S3 provider against a local Garage container.
- **Checks:** `cargo test` in `backend/`, `npm run lint` in `frontend/`.

The design docs describe every feature, its data model and its API routes. Start at [docs/README.md](./docs/README.md).

### Project Layout

```
addon/      Garry's Mod addon (GLua)
assets/     Logo and images for this README
backend/    API server (Rust, Actix Web, sqlx); migrations in backend/migrations
docs/       Design docs: data models, routes, permissions, behavior
frontend/   Website and loading screen (Next.js, TypeScript)
```

## Contributing

Issues and pull requests are welcome. Before working on something large, please open an issue to discuss it. Read the relevant doc in [docs/](./docs/README.md) first, and update it if your change affects behavior or routes.

## Security

Please don't report vulnerabilities in public issues. See [SECURITY.md](./SECURITY.md).

## License

[MIT](./LICENSE)
