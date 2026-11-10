<div align="center">
    <img src="./assets/logo.svg" width=250 height=250 />
    </br></br>
    <h1>GarryNet</h1>
    <p>GarryNet is an all-in-one web solution for Garry's Mod communities. Everything from loading screens to server-syncing to store functionality.</p>
</div>

## Features

### Community Unification

One deployment per community. A homepage with your name, logo, splash image, external links and key figures, plus a loading screen for your Garry's Mod servers with a server select. All editable from the admin dashboard.

### Forums

Sections, categories and threads with markdown posts, likes/dislikes and user reputation. Includes a support system with tickets and an FAQ.

### Store

Sell in-game rewards (usergroups, weapons, DarkRP money) per server. Payments go through Stripe Checkout, with discounts, receipts and refunds. Purchases are delivered in-game automatically by the addon.

### User Management

Steam login, profiles, online status, and usergroups with permission bits and custom colors, icons and effects.

### Server Management

Register servers on the website, link them with a one-time access key, and see their status, gamemode and map.

### Player Management

Per-server player records (playtime and last seen) synced from the addon every minute.

## Deployment

For each of the methods below, you will a Steam API Key: https://steamcommunity.com/dev

Configure your enviornment variables according to the provided `.env.example` samples. The dockerfile reads from `.env` in each component.

### Docker (Recommended)

1. Clone this repository onto your web server
    - Optionally: remove the addon folder or move it to your Garry's Mod server's `/addons` folder
2. Setup the required enviornment variables
3. Run `docker compose up -d --build`
    - `.env` is read from `backend/` and `frontend/` if present. `DATABASE_URL`, `BIND_ADDR` and the frontend's `API_URL` are overridden to use the compose network.
    - `NEXT_PUBLIC_API_URL` is baked in at build time, so re-run with `--build` after changing it.

### Manually

Requires PostgreSQL, Rust, and Node.js. The backend and frontend are separate; start the backend first.

#### Backend

1. Create a PostgreSQL database and copy `backend/.env.example` to `backend/.env`
2. Set `DATABASE_URL`, `JWT_SECRET` and `STEAM_API_KEY`. Stripe keys are optional (store checkout is disabled without them)
3. Run `cargo run --release` in `backend/`. Migrations are applied on startup

#### Frontend

1. Copy `frontend/.env.example` to `frontend/.env` and point `API_URL` and `NEXT_PUBLIC_API_URL` at the backend
2. Run `npm install && npm run build && npm start` in `frontend/`

### Addon

1. GarryNet needs the variable `garrynet_api_url` to be set in `garrysmod/cfg/server.cfg`.
    ```
    garrynet_api_url "http://loopback.gmod:8080"
    ```
2. Place the addon folder into your server's `garrysmod/addons` folder
    - Optionally: rename to `gnet`
3. Restart your server
4. Ensure the GarryNet addon can communicate with the backend
5. Link the server using the standard link flow 

## Developing

Read the [Design Docs](./docs/DESIGN.md).