# Addon Details

Part of [GarryNet Design](README.md). Servers and access keys are defined in [SERVERS.md](SERVERS.md).

The GarryNet Garry's Mod addon is responsible for communicating with GarryNet. It will update player data, fetch unrealized store requests, config changes, etc.

**Status:** linking, unlinking, the per-minute heartbeat (server details, player records and gamemode stats), store delivery, the `!gstore` store menu, store chat messages and ban enforcement work. Config changes are not implemented yet.

## Installation and Configuration

Copy or symlink `addon/` to `garrysmod/addons/garrynet`. In `server.cfg`, set the backend's public URL (its `BACKEND_URL`):

```
garrynet_api_url "https://api.example.com"
```

Use `https://`. The access key and the server's JWT travel in the `Authorization` header, so over plain HTTP anyone on the path can steal them and act as the server (report players, fetch and acknowledge store deliveries). The addon still works with `http://`, but warns about it in the server console (on startup, when the URL changes and when linking), in `garrynet_status` and in the `!gnet` menu. Loopback URLs (`localhost`, `127.x.x.x`, `[::1]`) don't trigger the warning.

Garry's Mod refuses HTTP requests to local and private addresses. When the backend runs on the same machine or network (e.g. in development), start the server with `-allowlocalhttp`.

Link state (server id, JWT and server name) is persisted in `garrysmod/data/garrynet/state.json`. The JWT is a secret and is never sent to clients. The last ban list is kept in `bans.json` beside it (see [Ban Enforcement](#ban-enforcement)).

## Structure

Files live in `addon/lua/garrynet/`, and are loaded in order by `addon/lua/autorun/garrynet.lua`. The `sh_`, `sv_` or `cl_` prefix decides the realm.

| File | Purpose |
| --- | --- |
| `sh_config.lua` | Constants, the `garrynet_api_url` convar, net message names |
| `sh_util.lua` | Logging, access key format check |
| `sh_status.lua` | Link status sent to the menu (read/write over net) |
| `sh_store_menu.lua` | Store menu contents sent to a player (read/write over net) |
| `sv_storage.lua` | Persisted server id and JWT |
| `sv_http.lua` | HTTP wrapper for `/gnet/server/...` requests, JWT rotation |
| `sv_link.lua` | Link (access key to JWT), unlink, out-of-sync handling |
| `sv_interval.lua` | Player session tracking, per-minute heartbeat, force sync |
| `sv_stats.lua` | Gamemode stats: providers, sampling, counters |
| `sv_store_rewards.lua` | Store rewards: usergroup adapters, timed usergroups, weapons, revoking |
| `sv_store.lua` | Store delivery: fetch the fulfillment queue, apply, acknowledge |
| `sv_bans.lua` | Ban enforcement: fetch the ban list, refuse and kick banned players |
| `sv_net.lua` | Net handlers for the menu; every one checks for superadmin |
| `sv_store_menu.lua` | `!gstore`: fetch a player's purchases and the store link, send them the menu |
| `sv_store_messages.lua` | Repeating store chat messages: settings, timer |
| `sv_commands.lua` | `!gnet` and `!gstore` chat commands, `garrynet_*` console commands |
| `cl_ui.lua` | The menus' shared look: colors, fonts, overlay, card, buttons |
| `cl_net.lua` | Client net handlers and senders, chat notices |
| `cl_menu.lua` | The `!gnet` menu |
| `cl_store_menu.lua` | The `!gstore` menu |

## !gnet

A superadmin can execute `!gnet` in chat and is presented with a dark-blue menu, with a darkened and blurred background. Other players are refused. Escape, the close button or a click outside the panel closes it.

**Not Linked**
Prompt the superadmin for the generated server access key (`gnet_` followed by 64 hex characters). It is sent to `/gnet/server/link` as the authorization header and exchanged for a JWT the server authenticates with. The backend then revokes the access key: keys only work once.

**Linked**
Shows the server's GarryNet name and id, and the last sync. "Sync now" forces an interval update; "Unlink" (click twice to confirm) revokes the JWT and forgets it locally.
Below that are the [store chat messages](#store-chat-messages) settings.

### Store chat messages

A superadmin can have a chat message advertising the store sent to every player on a timer. The `!gnet` menu (when
linked) has a switch to turn them on, the interval in minutes (default 10, 1 to 1440) and the message (at most 200
characters; line breaks become spaces). Nothing changes until "Save"; the menu says when there are unsaved changes.
Settings are saved in `garrysmod/data/garrynet/store_messages.json` and survive restarts. Defaults and limits are
`StoreMessages` in `sh_config.lua`.

- Before each message the addon asks GarryNet whether the server has an enabled store (`GET /gnet/server/{id}/store`
  without `steam_id`). If it doesn't, or GarryNet can't be reached, the message is skipped and the menu and
  `garrynet_status` say why. Nothing is sent while the server is empty.
- The menu shows when the next message goes out. Saving restarts the countdown only when messages are turned on or
  the interval changes, so editing the message doesn't put the next one off.
- Messages appear in chat with the `[GarryNet]` prefix.

### Confirmation

Every superadmin in game gets a chat notice when the server is linked, when the first heartbeat is accepted, and when the server is unlinked or falls out of sync. On the website, the server's status in Admin > Servers goes from "Not linked" to "Online" (after the first heartbeat, which is sent right after linking), with its gamemode and map filled in.

### Console commands

These run from the server console, or from a superadmin's own console:

- `garrynet_status`: link status, last sync, and the last store and ban list syncs
- `garrynet_link <access key>`: link without the menu
- `garrynet_unlink`
- `garrynet_sync`: send a heartbeat now
- `garrynet_store_sync`: fetch and deliver store purchases now
- `garrynet_store_grants <SteamID64>`: a player's store usergroups and weapons

## !gstore

Any player can execute `!gstore` in chat to open the *GarryNet Store Menu*, in the same style as `!gnet`. It lists
their 10 most recent purchases on this server, newest first, each with the item name, price, date, and one status:
Delivered, Awaiting delivery, Delivery failed, Refunded, Disputed or Revoked. A line under the list explains when
something is still awaiting delivery or failed. "Open store" opens the server's store page on the website
(`gui.OpenURL`) and closes the menu.

- Purchases are matched by the player's SteamID64 (the one captured at checkout), and only this server's are shown.
- If the server isn't linked, has no store, or its store is disabled, the player gets a chat notice ("This server
  doesn't have a store set up.") instead of the menu. If GarryNet can't be reached, they're told to try again and the
  error is logged in the server console.
- Each opening asks GarryNet (nothing is cached), so players can open it once every `StoreMenuCooldown` (5) seconds.

## Intervals

A GNet interval occurs every minute and syncs server and player information to GarryNet through `POST /gnet/server/{SERVER_ID}/heartbeat`, authorized with the server JWT. It also runs 5 seconds after the map loads.

The first successful sync after linking is the server's first heartbeat, from which the backend fills in the server's `gamemode` and `map`. A 401 means the server and GarryNet are out of sync (the server was unlinked or deleted on the website, its access key was regenerated, or its token expired). The addon then forgets its credentials and tells superadmins; the access key must be regenerated and the server linked again.

The addon tracks each human player's session (bots are skipped) and reports playtime in whole minutes, keeping the remainder for the next heartbeat. Unreported playtime survives failed heartbeats, so nothing is lost while GarryNet is unreachable. A disconnected player is reported once more, then dropped.

### Gamemode Stats

GNet records gamemode-specific stats per server for every player, sent with each player in the heartbeat and stored in
their UserPlayer's `stats` (see [USERS.md](USERS.md)). Keys are lowercase and prefixed with the gamemode's name, so the
website can label `darkrp_money` as "Money".

| Gamemode | Key | Kind | What |
| --- | --- | --- | --- |
| DarkRP | `darkrp_money` | value | Money (sent once DarkRP has loaded it) |
| DarkRP | `darkrp_job` | value | Current job; the last one once the player leaves |
| Sandbox | `sandbox_props_spawned` | counter | Props spawned |
| Sandbox | `sandbox_tools_used` | counter | Tool gun clicks (left, right, reload) that reach `CanTool` |

- **Values** are sampled every heartbeat and when the player leaves, and replace what GarryNet has. A value the gamemode
  doesn't have at the moment keeps its last known value.
- **Counters** are counted as events happen and only the increase since the last accepted heartbeat is sent; GarryNet
  adds it to the stored total. Like playtime, unreported counts survive failed heartbeats.
- Sandbox stats are only collected on Sandbox itself, not on gamemodes derived from it such as DarkRP.
- When a heartbeat reports a different gamemode than the server had (ignoring case), GarryNet clears the stats of every
  player on that server before recording the heartbeat, so no player keeps the old gamemode's stats. Players online then
  get the new gamemode's stats straight away; the rest get them on their next visit.

Other addons can add stats for their gamemode:

```lua
GarryNet.Stats.Register("murder", {
    active = function() return engine.ActiveGamemode() == "murder" end,
    sample = function(ply) return { murder_wins = ply:GetNWInt("Wins") } end, -- values; optional
})
GarryNet.Stats.Count(ply, "murder_rounds_played") -- counters, e.g. from a round hook
```

## Ban Enforcement

When **Keep banned users off game servers** is on in [Admin > Community](ADMIN.md#admincommunity), linked servers keep out players banned on GarryNet (see [User Punishment](USERS.md#user-punishment)). It's off by default. Suspensions don't apply in-game.

- **When:** the addon fetches the ban list every minute (`IntervalSeconds`), 5 seconds after the map loads, and right after linking. Garry's Mod's `CheckPassword` hook has to answer immediately, so connections are checked against this list rather than asking GarryNet each time: a new ban or unban takes up to a minute to apply.
- **Connecting:** a banned player is refused in `CheckPassword` with a message giving the reason, when the ban ends (or that it's permanent) and where to appeal (the website's `/support`). Players who aren't banned are left to other hooks and the server password, as usual.
- **Already playing:** after each fetch, banned players on the server are kicked with the same message. This covers bans issued while they play and turning enforcement on.
- **Expiry:** each ban carries its end, so it stops applying on time between fetches.
- **GarryNet unreachable:** the last list keeps being enforced, including across restarts (it's saved in `data/garrynet/bans.json`). Failures are logged once, like heartbeats.
- **Unlinking** (or a 401) forgets the list, so an unlinked server doesn't enforce stale bans.
- Refusals and kicks are logged in the server console. Bans are matched by the connecting account's SteamID64 only; Steam Family Sharing borrowers aren't matched to the lender.

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
      "minutes": 1,
      "stats": { "darkrp_money": 5000, "darkrp_job": "Citizen" },
      "counters": { "sandbox_props_spawned": 3 }
    }
  ]
}
```

`connected_at` (unix seconds) is when the session began and becomes the player's `last_logon`. `minutes` is playtime not reported before, added to `time_played`. `stats` (optional) replaces those keys of the
player's `stats`: numbers, booleans, or strings (truncated to 128 characters). `counters` (optional) are whole numbers
added to those keys (a stored value that isn't a number counts as 0), at most 1,000,000 per heartbeat. Stats not mentioned
are kept. Keys must match `[a-z0-9_]{1,64}`, and up to 32 of each are taken; invalid entries are dropped rather than
failing the heartbeat. A player's record is created on their first visit and linked to their GarryNet account if they have one. At most 256 players per heartbeat. Returns `{ "server": {...} }`, plus `"token"` when the JWT was rotated.

**POST `/gnet/server/{SERVER_ID}/unlink`**: `Authorization: Bearer <JWT>`. Unlinks the server and revokes its token and any access key. Returns 204.

**GET `/gnet/server/{SERVER_ID}/bans`**: `Authorization: Bearer <JWT>`. Users banned on GarryNet right now, by SteamID64. `ends` is unix seconds, or null for a permanent ban. When the community doesn't keep banned users off its servers, `enforce` is false and `bans` is empty. `appeal_url` is the website's support page, for the kick message.

```json
{
  "enforce": true,
  "appeal_url": "https://example.com/support",
  "bans": [{ "steam_id": "76561197960287931", "reason": "Aimbot on DarkRP", "ends": 1795000000 }]
}
```

**GET `/gnet/server/{SERVER_ID}/store?steam_id=`**: `Authorization: Bearer <JWT>`. What a player's `!gstore` menu
shows: the server's store (`null` if it has none or it's disabled) and the player's 10 most recent purchases on this
server, newest first, matched by the fulfillment's SteamID64. Without `steam_id`, `purchases` is empty (used to check
there's a store before a store chat message). `price` is in the currency's minor unit, `status` is the
receipt's, `delivery` the fulfillment's, and `purchased_at` is unix seconds. 400 if `steam_id` isn't a SteamID64.

```json
{
  "store": { "id": 1, "name": "DarkRP Store", "url": "https://example.com/store/1" },
  "purchases": [
    { "id": 7, "item_name": "VIP (30 days)", "price": 499, "currency": "usd", "status": "PAID",
      "delivery": "FULFILLED", "purchased_at": 1762900000 }
  ]
}
```

### Store fulfillments

See [STORE.md](STORE.md) for the model. The addon should fetch work when a player joins (with their
`steam_id`) and on its interval, apply each fulfillment's `rewards` and report back.

**GET `/gnet/server/{SERVER_ID}/fulfillments?steam_id=`**: `Authorization: Bearer <JWT>`. Fulfillments the
server has to act on, oldest first: `PENDING` and retryable `FAILED` ones to deliver, and `REVOKED` ones
whose rewards were delivered (`fulfilled_at` set, `revocation_applied_at` null) to take back (usergroups and
weapons; not money). `steam_id` narrows it to one player.

```json
[
  {
    "id": 1, "receipt_id": 1, "buyer_id": 2, "steam_id": "76561197960287931", "item_id": 2, "server_id": 1,
    "rewards": [
      { "type": "usergroup", "usergroup": "vip", "duration_seconds": 2592000 },
      { "type": "darkrp_money", "amount": 5000 },
      { "type": "weapon", "class": "weapon_crowbar" }
    ],
    "status": "PENDING", "attempts": 0, "last_error": null,
    "created_at": "...", "fulfilled_at": null, "revoked_at": null, "revocation_applied_at": null
  }
]
```

**POST `/gnet/server/{SERVER_ID}/fulfillments/{FULFILLMENT_ID}/ack`**: `Authorization: Bearer <JWT>`. Body
`{ "result": "fulfilled" }`, `{ "result": "failed", "error": "player not in game" }` or `{ "result": "revoked" }`.
Idempotent: repeating a report changes nothing. Returns the fulfillment as it stands; 404 if it isn't this server's.

### Delivery in the addon

- **When:** every minute (`Store.PollSeconds`), and `Store.JoinDelay` (10) seconds after a player joins, filtered to them.
  Only players who are online get their purchase; revocations are applied whether or not the player is online.
- **All or nothing:** every reward is validated before any is applied (the usergroup exists in the admin mod and isn't
  protected, the server runs DarkRP for money, the weapon class is known). A failure is acknowledged as `failed` with the
  reason, superadmins and the buyer get a chat notice, and it's retried with backoff (1, 2, 4... minutes, at most 60), so
  fixing the server gives it a chance before GarryNet stops offering it after 20 attempts (staff with Store Finance can
  reset it from the receipts page once it's fixed). DarkRP money waits, without
  failing, until DarkRP has loaded the player's data.
- **Exactly once:** an applied fulfillment is written to `data/garrynet/store_pending.json` before it's acknowledged and
  only forgotten once GarryNet confirms. If the acknowledgement is lost, the next fetch re-acknowledges instead of applying
  again. A Lua error part-way through applying is reported as `failed` and never retried automatically.
- **Usergroups:** set through an adapter: ULX (ULib, persistent, works for offline players) when installed, else
  `Player:SetUserGroup`, which GarryNet re-applies on every join. `GarryNet.Rewards.SetUsergroupAdapter` plugs in other admin
  mods. Purchases of the same group stack (each duration starts when the previous one ends); with several store groups the
  most recently bought one wins. A store group only replaces a group in `Store.OverridableGroups` (default `user`) or one
  the store set itself, so staff are never demoted; their grant still counts if they later return to `user`. When the
  last store group ends or is revoked, the player goes back to `Store.DefaultGroup`. `Store.ProtectedGroups` are never granted.
- **Weapons** are recorded and given back on every spawn (1 second after, skipped while DarkRP-arrested).
- Grants are kept in `data/garrynet/store_grants.json`, per player and per fulfillment, so a revocation removes exactly that
  purchase's usergroups and weapons.

Config changes are still to be defined.
