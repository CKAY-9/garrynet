<div align="center">
    <img src="./assets/logo.svg" width=250 height=250 />
    </br></br>
    <h1>GarryNet</h1>
    <p>GarryNet is an all-in-one web solution for Garry's Mod communities. Everything from loading screens to server-syncing to store functionality.</p>
</div>

## Features

### Community Unificaiton

### Forums

### Store

### User Management

### Server Management

### Player Management

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

#### Frontend

#### Backend

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