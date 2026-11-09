# Addon Details

Part of [GarryNet Design](DESIGN.md). Servers and access keys are defined in [SERVERS.md](SERVERS.md).

The GarryNet Garry's Mod addon is responsible for communicating with GarryNet. It will update player data, fetch unrealized store requests, config changes, etc.

**Status:** linking, unlinking and the per-minute heartbeat (server details and player records) work. Store requests and config changes are not implemented yet.

## Installation and Configuration

Copy or symlink `addon/` to `garrysmod/addons/garrynet`. In `server.cfg`, set the backend's public URL (its `BACKEND_URL`):

```
garrynet_api_url "https://api.example.com"
```

Garry's Mod refuses HTTP requests to local and private addresses. When the backend runs on the same machine or network (e.g. in development), start the server with `-allowlocalhttp`.

Link state (server id, JWT and server name) is persisted in `garrysmod/data/garrynet/state.json`. The JWT is a secret and is never sent to clients.

## Structure

Files live in `addon/lua/garrynet/`, and are loaded in order by `addon/lua/autorun/garrynet.lua`. The `sh_`, `sv_` or `cl_` prefix decides the realm.

| File | Purpose |
| --- | --- |
| `sh_config.lua` | Constants, the `garrynet_api_url` convar, net message names |
| `sh_util.lua` | Logging, access key format check |
| `sh_status.lua` | Link status sent to the menu (read/write over net) |
| `sv_storage.lua` | Persisted server id and JWT |
| `sv_http.lua` | HTTP wrapper for `/gnet/server/...` requests, JWT rotation |
| `sv_link.lua` | Link (access key to JWT), unlink, out-of-sync handling |
| `sv_interval.lua` | Player session tracking, per-minute heartbeat, force sync |
| `sv_net.lua` | Net handlers for the menu; every one checks for superadmin |
| `sv_commands.lua` | `!gnet` chat command, `garrynet_*` console commands |
| `cl_net.lua` | Client net handlers and senders, chat notices |
| `cl_menu.lua` | The `!gnet` menu |

## !gnet

A superadmin can execute `!gnet` in chat and is presented with a dark-blue menu, with a darkened and blurred background. Other players are refused. Escape, the close button or a click outside the panel closes it.

**Not Linked**
Prompt the superadmin for the generated server access key (`gnet_` followed by 64 hex characters). It is sent to `/gnet/server/link` as the authorization header and exchanged for a JWT the server authenticates with. The backend then revokes the access key: keys only work once.

**Linked**
Shows the server's GarryNet name and id, and the last sync. "Sync now" forces an interval update; "Unlink" (click twice to confirm) revokes the JWT and forgets it locally.

### Confirmation

Every superadmin in game gets a chat notice when the server is linked, when the first heartbeat is accepted, and when the server is unlinked or falls out of sync. On the website, the server's status in Admin > Servers goes from "Not linked" to "Online" (after the first heartbeat, which is sent right after linking), with its gamemode and map filled in.

### Console commands

These run from the server console, or from a superadmin's own console:

- `garrynet_status`: link status and last sync
- `garrynet_link <access key>`: link without the menu
- `garrynet_unlink`
- `garrynet_sync`: send a heartbeat now

## Intervals

A GNet interval occurs every minute and syncs server and player information to GarryNet through `POST /gnet/server/{SERVER_ID}/heartbeat`, authorized with the server JWT. It also runs 5 seconds after the map loads.

The first successful sync after linking is the server's first heartbeat, from which the backend fills in the server's `gamemode` and `map`. A 401 means the server and GarryNet are out of sync (the server was unlinked or deleted on the website, its access key was regenerated, or its token expired). The addon then forgets its credentials and tells superadmins; the access key must be regenerated and the server linked again.

The addon tracks each human player's session (bots are skipped) and reports playtime in whole minutes, keeping the remainder for the next heartbeat. Unreported playtime survives failed heartbeats, so nothing is lost while GarryNet is unreachable. A disconnected player is reported once more, then dropped.

## Server Tokens

The JWT is valid for 30 days and is rolling: once it is an hour old, the next heartbeat response carries a replacement, which the addon stores. Tokens carry the server's `token_version`, which the backend bumps when the server links, unlinks or gets a new access key. That revokes every earlier token. A server offline for longer than 30 days must be linked again.

## Backend Routes

All under `/gnet/server`. Errors are `{ "error": "..." }`.

**POST `/gnet/server/link`**: `Authorization: Bearer <access key>` (the `Bearer ` prefix is optional). Marks the server linked and revokes the key.

```json
{ "server_id": 1, "token": "<JWT>", "server": { "id": 1, "name": "DarkRP", "...": "..." } }
```

401 if no server has that key, including a key that has already been used.

**POST `/gnet/server/{SERVER_ID}/heartbeat`**: `Authorization: Bearer <JWT>`.

```json
{
  "gamemode": "DarkRP",
  "map": "rp_downtown_v4c",
  "players": [
    {
      "steam_id": "76561197960287930",
      "name": "Garry",
      "usergroup": "superadmin",
      "connected_at": 1762700000,
      "minutes": 1
    }
  ]
}
```

`connected_at` (unix seconds) is when the session began and becomes the player's `last_logon`. `minutes` is playtime not reported before, added to `time_played`. A player's record is created on their first visit and linked to their GarryNet account if they have one. At most 256 players per heartbeat. Returns `{ "server": {...} }`, plus `"token"` when the JWT was rotated.

**POST `/gnet/server/{SERVER_ID}/unlink`**: `Authorization: Bearer <JWT>`. Unlinks the server and revokes its token and any access key. Returns 204.

Pending store requests and config changes are still to be defined.
