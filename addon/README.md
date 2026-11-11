# GarryNet Addon

Garry's Mod addon that connects a server to a GarryNet deployment. See [docs/ADDON.md](../docs/ADDON.md),
[docs/SERVERS.md](../docs/SERVERS.md) and [docs/USER_STORIES.md](../docs/USER_STORIES.md) for the design.

**Status:** linking, unlinking, the per-minute heartbeat (server details, player records, gamemode stats) and store delivery work. Pushing config changes from the website is not implemented yet.

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

## Gamemode Stats

Each heartbeat carries per-player stats for the running gamemode: DarkRP money and job, and Sandbox props spawned and tool uses.
Other addons can register their own with `GarryNet.Stats.Register` and `GarryNet.Stats.Count`; see
[docs/ADDON.md](../docs/ADDON.md#gamemode-stats).

## Store

Purchases from the website's store are delivered automatically: the addon checks for new ones every minute and when a
player joins, and buyers get a chat notice. Players who are offline get their purchase the next time they join.

- **Usergroups** are set with ULX when it's installed (persisted by ULib), otherwise with `SetUserGroup` and re-applied on
  every join. Timed usergroups stack and end on time. Only players in `user` are moved into a store usergroup, so staff
  are never demoted; edit `OverridableGroups` in `sh_config.lua` to change that. `superadmin`, `admin`, `owner` and
  `developer` are never granted. The usergroup must exist in ULX, or the delivery fails and superadmins are told.
- **DarkRP money** is added once the player's DarkRP data has loaded.
- **Weapons** are permanent and given back on every spawn.
- A short sound plays for the buyer on delivery (`Store.RewardSound` in `sh_config.lua`, `""` to disable).
- **Refunds** take usergroups and weapons back; money stays.

Other admin mods can plug in with `GarryNet.Rewards.SetUsergroupAdapter` (see `sv_store_rewards.lua`). Console:
`garrynet_store_sync` delivers now, `garrynet_store_grants <SteamID64>` lists a player's store usergroups and weapons.
State is kept in `garrysmod/data/garrynet/store_grants.json` and `store_pending.json`; don't delete them while purchases
are being delivered.

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
  sv_stats.lua              gamemode stats: providers, sampling, counters
  sv_store_rewards.lua      store rewards: usergroup adapters (ULX/plain), timed groups, weapons, revoking
  sv_store.lua              store delivery: fetch the queue, apply, acknowledge
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
- `GET /gnet/server/{SERVER_ID}/fulfillments`: store purchases to deliver or take back
- `POST /gnet/server/{SERVER_ID}/fulfillments/{ID}/ack`: reports `fulfilled`, `failed` or `revoked`

Payloads are documented in [docs/ADDON.md](../docs/ADDON.md#backend-routes).
