# GarryNet Addon

Garry's Mod addon that connects a server to a GarryNet deployment. See [docs/ADDON.md](../docs/ADDON.md),
[docs/SERVERS.md](../docs/SERVERS.md) and [docs/USER_STORIES.md](../docs/USER_STORIES.md) for the design.

**Status:** linking, unlinking and the per-minute heartbeat (server details, player records) work. Store requests and config changes are not implemented yet.

## Install

Copy or symlink this folder to `garrysmod/addons/garrynet`, then set the backend URL in `server.cfg`:

```
garrynet_api_url "https://api.example.com"
```

This is the backend's public URL (its `BACKEND_URL`). If the backend is on the same machine or a private network, start the
server with `-allowlocalhttp`, since Garry's Mod blocks HTTP to local addresses otherwise.

To link, register the server on the website (Admin > Servers), then a superadmin runs `!gnet` in chat and pastes the access key.
Superadmins get a chat notice once it's linked and once the first heartbeat is accepted; the website then shows the server as
Online. From the server console: `garrynet_link <key>`, `garrynet_status`, `garrynet_sync`, `garrynet_unlink`.

## Layout

```
addon.json
lua/autorun/garrynet.lua    bootstrap: global table, ordered file loader (realm picked by prefix)
lua/garrynet/
  sh_config.lua             constants, convar, net message names
  sh_util.lua               logging, access key format check
  sh_status.lua             link status sent to the menu (net read/write)
  sv_storage.lua            persisted server id + JWT (garrysmod/data/garrynet/)
  sv_http.lua               HTTP wrapper for /gnet/server/... requests, JWT rotation
  sv_link.lua               link (access key -> JWT) and unlink
  sv_interval.lua           player sessions, per-minute heartbeat, force sync
  sv_net.lua                net handlers for the menu (superadmin-checked)
  sv_commands.lua           !gnet chat command, garrynet_* console commands
  cl_net.lua                client net handlers and senders
  cl_menu.lua               dark-blue blurred menu (not linked / linked views)
```

Add new files to the `FILES` list in `lua/autorun/garrynet.lua`; the prefix (`sh_`, `sv_`, `cl_`) decides the realm.

## Backend routes

- `POST /gnet/server/link`: access key in the authorization header, returns the server id and JWT
- `POST /gnet/server/{SERVER_ID}/heartbeat`: gamemode, map and players; may return a rotated JWT
- `POST /gnet/server/{SERVER_ID}/unlink`: revokes the JWT

Payloads are documented in [docs/ADDON.md](../docs/ADDON.md#backend-routes).
