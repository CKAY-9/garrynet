# Users

Part of [GarryNet Design](README.md).

Users are authenticated using Steam. Everyone logs in with their Steam account and GarryNet stores each user in its own table entry.

## User

```
id: number
username: string
email?: string // optional; Steam does not provide it, so it must be connected later
avatar_url: string
steam_id: string
bio: string
reputation: number // total likes minus dislikes across all of the user's forum posts
status: UserStatus
joined: utc timestamp
last_seen: utc timestamp // updated when they access a route/resource
```

Profile details (username, avatar) come from Steam and are refreshed each time the user logs in. Avatars always come from Steam; users can't upload their own.

**Planned:** a "Sync with Steam" button on the profile that refreshes them without logging out and back in.

## User Status

- Online: set when the user accesses a route/resource
- Idle: set after 5 minutes without access
- Offline: set after 10 minutes of being idle (15 minutes since last access)

```
UserStatus:
ONLINE,
IDLE,
OFFLINE
```

## User Punishment

**Planned.** Not implemented yet: there is no punishments table or route. Until then, removing a user's `Site Access` (through their usergroups) works as a ban.

Tracks website bans, suspensions, and warnings. These are separate from Player Punishments, which apply in-game.

```
id: number
user_id: number
reason: string
punishment: "ban" | "suspend" | "warn"
issuer_id: number
created_at: utc timestamp
ends?: utc timestamp // when this punishment stops being enforced; null/empty if permanent
active: boolean // true = currently enforced; false = kept for history only
```

## User Player

A UserPlayer is a per-server record linking a player to a GarryNet user. For example, a user has one UserPlayer for the community DarkRP server and another for the Sandbox server. The GarryNet addon updates UserPlayer records by sending HTTP requests to the GarryNet backend.

```
id: number
server_id: number // id of the Garry's Mod Server
steam_id: string
user_id?: number // GarryNet user id (not the Steam id); empty if the player has no GarryNet account
ingame_name: string
last_logon: utc timestamp // equals first_joined until the player returns
first_joined: utc timestamp // also serves as created_at
ingame_usergroup: string // defaults to "user" if none is provided
stats: json // gamemode-specific data, e.g. `darkrp_money`; updated every heartbeat (see "Gamemode Stats" in ADDON.md)
time_played: number // in minutes; the addon updates it every minute
```

## Player Punishment

Player Punishments are in-game punishments such as kicks, bans, jails, and warnings.

**Planned.** Not implemented yet.

## User Searching

Implemented on the frontend at `/users/search` (see [Site Design](SITE_DESIGN.md)). The side navigation has a search box that searches by name and opens the results there. With no filters, the page shows a few random users.

- GET `/users/search?name=&last_activity=&id=&usergroup=&sort_by=&limit=`: users matching every given filter. No login required. Blank values are ignored.
    - name: case-insensitive part of the username
    - last_activity: how recently the user was seen: `online`, `hour`, `day`, `week`, or `month`
    - id: GarryNet user id (matches at most one user)
    - usergroup: id of a usergroup the users belong to
    - sort_by: `reputation+` (most first; send the `+` as `%2B`), `reputation-` (least first), or `posts` (most forum posts first). When omitted, exact and prefix name matches come first, or the order is random if no name is given.
    - limit: 1 to 100, default 24

    Each result has the user's public profile fields (no email or bio), `post_count`, and `usergroups`.

- GET `/usergroups`: every usergroup's display details (`id`, `name`, `effect`, `color`, `icon_url`), used for the usergroup filter. No login required.
