# Addon Details

Part of [GarryNet Design](DESIGN.md). Servers and access keys are defined in [SERVERS.md](SERVERS.md).

The GarryNet Garry's Mod addon is responsible for communicating with GarryNet. It will update player data, fetch unrealized store requests, config changes, etc.

**Status:** the addon in [`addon/`](../addon) is a scaffold. Its structure, module APIs and `TODO`s exist, but nothing is implemented yet, and the backend has no `/gnet` routes. Everything below describes the intended behavior.

## Installation and Configuration

Copy or symlink `addon/` to `garrysmod/addons/garrynet`. Set the backend URL in `server.cfg`:

```
garrynet_api_url "https://<site>/api"
```

The public API prefix is not decided yet. Link state (server id and JWT) is persisted in `garrysmod/data/garrynet/`. The JWT is a secret and is never sent to clients.

## Structure

Files live in `addon/lua/garrynet/`, and are loaded in order by `addon/lua/autorun/garrynet.lua`. The `sh_`, `sv_` or `cl_` prefix decides the realm.

| File | Purpose |
| --- | --- |
| `sh_config.lua` | Constants, the `garrynet_api_url` convar, net message names |
| `sh_util.lua` | Logging, access key format check |
| `sv_storage.lua` | Persisted server id and JWT |
| `sv_http.lua` | HTTP wrapper for `/gnet/server/...` requests |
| `sv_link.lua` | Link (access key to JWT) and unlink |
| `sv_interval.lua` | Per-minute sync, force sync |
| `sv_net.lua` | Net handlers for the menu; every one checks for superadmin |
| `sv_commands.lua` | `!gnet` chat command |
| `cl_net.lua` | Client net handlers and senders |
| `cl_menu.lua` | The `!gnet` menu |

## !gnet

A superadmin should be able to execute `!gnet` in chat and be presented with a dark-blue menu, with a darkened and blurred background. Other players are refused.

**Not Linked**
Prompt the superadmin for the generated server access key (`gnet_` followed by 64 hex characters). It will be sent to `/gnet/server/link` with it as an authorization header. Exchanging should result in a JWT for the server that it can use to authenticate. The backend then revokes the access key.

**Linked**
Display options for unlinking (revoking/removing JWT and access key, if it still exists), force interval update

## Intervals

A GNet interval should occur every minute and sync player information to GarryNet. This is done by HTTP requests to `/gnet/server/{SERVER_ID}/[route]` on the backend, authorized with the server JWT.

The first successful sync after linking is the server's first heartbeat, from which the backend fills in the server's `gamemode` and `map`. A 401 means the server and GarryNet are out of sync; the access key must be regenerated and the server linked again.

## Backend Routes

- POST `/gnet/server/link`: access key as the authorization header, returns the server id and JWT
- `/gnet/server/{SERVER_ID}/[route]`: heartbeat, player sync, pending store requests, config changes

Routes other than linking, and all payloads, are still to be defined.
