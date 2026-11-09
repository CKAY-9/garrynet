# GarryNet Addon

Garry's Mod addon that connects a server to a GarryNet deployment. See [docs/ADDON.md](../docs/ADDON.md),
[docs/SERVERS.md](../docs/SERVERS.md) and [docs/USER_STORIES.md](../docs/USER_STORIES.md) for the design.

**Status: scaffold only.** The structure, module APIs and TODOs are in place; functionality is not implemented.

## Install

Copy or symlink this folder to `garrysmod/addons/garrynet`, then set the backend URL in `server.cfg`:

```
garrynet_api_url "https://<site>/api"
```

(The real API prefix is still to be confirmed.) A superadmin then runs `!gnet` in chat.

## Layout

```
addon.json
lua/autorun/garrynet.lua    bootstrap: global table, ordered file loader (realm picked by prefix)
lua/garrynet/
  sh_config.lua             constants, convar, net message names
  sh_util.lua               logging, access key format check
  sv_storage.lua            persisted server id + JWT (garrysmod/data/garrynet/)
  sv_http.lua               HTTP wrapper for /gnet/server/... requests
  sv_link.lua               link (access key -> JWT) and unlink
  sv_interval.lua           per-minute sync, force sync
  sv_net.lua                net handlers for the menu (superadmin-checked)
  sv_commands.lua           !gnet chat command
  cl_net.lua                client net handlers and senders
  cl_menu.lua               dark-blue blurred menu (not linked / linked views)
```

Add new files to the `FILES` list in `lua/autorun/garrynet.lua`; the prefix (`sh_`, `sv_`, `cl_`) decides the realm.

## Backend routes

- `POST /gnet/server/link`: access key in the authorization header, returns the server JWT
- `/gnet/server/{SERVER_ID}/[route]`: authorized with the JWT (heartbeat, pending store requests, config)

Specific routes and payloads are not defined yet; the backend has no `/gnet` routes at this point.
