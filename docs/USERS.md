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
bio: string // plain text, up to 1000 characters; set by the user at /settings
reputation: number // total likes minus dislikes across all of the user's forum posts
status: UserStatus
joined: utc timestamp
last_seen: utc timestamp // updated when they access a route/resource
private: boolean // default false; set by the user at /settings
```

Profile details (username, avatar) come from Steam and are refreshed each time the user logs in. Avatars always come from Steam; users can't upload their own.

**Planned:** a "Sync with Steam" button on the profile that refreshes them without logging out and back in.

## Profile Settings

Users manage their own profile at `/settings` (User Settings): their bio and whether their profile is private. Banned users (no `Site Access`) can't; suspended users can. The sidebar links to it under the profile preview, and your own profile has an **Edit profile** button.

A **private** profile hides the user's bio, reputation, threads and posts from everyone but themselves and staff with `Admin Access` and `User Management` (the same staff who ban). Everyone else still sees the avatar, name, usergroups, status, join date, last seen and a ban in effect, plus "This profile is private". Their posts still show in the threads they're in, under their name; privacy only hides the list on the profile. In user search, private users' reputation and post count are hidden, and they sort last when sorting by either.

- PATCH `/users/me`: `{ bio?: string, private?: boolean }`; omitted fields stay as they are. The bio is trimmed. Returns the user like GET `/users/me`. 400 if nothing is given or the bio is too long, 403 without Site Access.

## Profile Activity

A profile lists small previews of the user's threads and posts (their replies), newest first: title, a plain-text excerpt, category, when, and reputation (likes minus dislikes; a thread's is its first post's). Viewers can narrow it to threads or posts, search the title and content, and sort by newest, oldest, or most or least reputation. The filters live in the URL (`/users/{id}?kind=&q=&sort=&page=`), 20 per page.

- GET `/users/{id}/posts?kind=&q=&sort=&limit=&offset=`: no login required, but 403 for a private profile unless it's the viewer's own or the viewer is staff (Admin Access and User Management). 404 if there's no such user. Blank values are ignored.
    - kind: `thread` (threads they started) or `post` (their replies); both by default
    - q: case-insensitive part of the thread title or the post content (up to 100 characters)
    - sort: `new` (default), `old`, `reputation+` (most first; send the `+` as `%2B`) or `reputation-`
    - limit: 1 to 50, default 20; offset: how many to skip

    ```json
    {
      "items": [{
        "is_thread": true, "post_id": 9, "thread_id": 5, "thread_title": "Server rules",
        "category_id": 8, "category_name": "Announcements",
        "excerpt": "# General ...", "created_at": "...", "edited": null, "reputation": 3
      }],
      "has_more": false
    }
    ```

    `excerpt` is the first 300 characters of the post's Markdown.

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

Website bans and suspensions, issued from a user's profile. These are separate from Player Punishments, which apply in-game. Warnings are **Planned**.

```
id: number
user_id: number
kind: BAN | SUSPEND
reason: string
issuer_id?: number // null once the issuer's account is deleted
created_at: utc timestamp
ends?: utc timestamp // when it stops being enforced; null if permanent (bans only; suspensions always end)
active: boolean // false once lifted early, or once an expired one is replaced; kept for history
lifted_by?: number // who lifted it early; null for the backend shell or a deleted account
lifted_at?: utc timestamp
lift_reason?: string // optional note from whoever lifted it
```

A punishment is **in effect** while it's active and hasn't ended. Expired punishments don't need a sweeper: they stop counting on their own. A user can have at most one ban and one suspension in effect, and both at once.

Punishments work by taking permissions away from the user's [effective permissions](PERMISSIONS.md#users-in-multiple-groups), without changing their usergroups, so every permission check honours them:

- **Ban:** every permission. The user is treated like any user without `Site Access` (see [AUTH.md](AUTH.md#banned-users)): only support, notifications and their receipts. If the community enables it in [Admin > Community](ADMIN.md#admincommunity), the addon also keeps them off the community's game servers (see [ADDON.md](ADDON.md#ban-enforcement)). Permanent or timed.
- **Suspension:** `User Posts` and `User Messages`. The user can still browse and buy, but can't post on the forums or send messages. Always timed. Suspended users can still join game servers.

### Banning and Suspending

Staff with `Admin Access` and `User Management` see **Ban** and **Suspend** buttons next to "View on Steam" on a user's profile, or **Unban** and **Unsuspend** while one is in effect. Each opens a dialog:

- Ban/Suspend asks for a reason (required, up to 500 characters, shown to the user) and a length: a preset (bans default to permanent, suspensions to 1 day) or "Until a date..." in the staff member's time zone.
- Unban/Unsuspend shows the current reason and end, and asks for an optional note, kept in the history but not shown to the user.

A ban in effect is public: everyone, logged out included, sees "Banned until ..." (or "permanently"), the reason, and when it was issued ("Banned on November 11, 2026 (3 days ago)") above the bio. Who issued it is never shown publicly. Suspensions aren't public: staff see one in effect (with who issued it), and so does the suspended user (without staff names). Staff also get a collapsible punishment history with who issued and lifted each one.

The user gets an ACCOUNT notification when banned or suspended (with the reason and end) and when it's lifted early (see [NOTIFICATIONS.md](NOTIFICATIONS.md)). Expiry isn't notified. Issuing and lifting are logged.

Rules:
- Nobody can ban, suspend, unban or unsuspend themselves (400).
- The target's usergroups must not grant any permission the staff member lacks (403 otherwise), the same rule as removing someone from a usergroup. Punishments are ignored here, so a banned admin still outranks a moderator, who can't unban them.
- One of each kind at a time: banning someone already banned is 409. To change a ban (e.g. its length), lift it and issue a new one.
- The end must be in the future.

The backend shell can lift them without these checks, e.g. to recover an admin banned by another: `user unban [USER_ID]` and `user unsuspend [USER_ID]`. `user show [USER_ID]` lists the punishments in effect.

### Routes

- GET `/users/{id}` (the public profile) includes `ban`: `{ reason, created_at, ends }` for a ban in effect, else null. No issuer. It also includes `private`; a private profile's `bio` and `reputation` are null unless the viewer may see them (see [Profile Settings](#profile-settings)).
- GET `/users/{id}/punishments`: the user's punishments, newest first (at most 100). Staff (Admin Access and User Management) get the full history; users get their own punishments in effect, with `issuer` and `lifted_by` null. 403 for anyone else, 404 (staff only) if there's no such user.

    ```json
    [{
      "id": 3, "kind": "BAN", "reason": "Aimbot on DarkRP",
      "created_at": "...", "ends": null, "in_effect": true,
      "issuer": { "id": 3, "username": "Mod" },
      "lifted_at": null, "lifted_by": null, "lift_reason": null
    }]
    ```

- POST `/users/{id}/punishments`: `{ kind: "BAN" | "SUSPEND", reason, ends?: utc timestamp }`; returns the punishment (201). Requires Admin Access and User Management. 409 if one of that kind is already in effect.
- POST `/users/{id}/punishments/lift`: `{ kind, reason? }`; lifts the one in effect and returns it. Same requirements. 404 if none is in effect.

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

    Each result has the user's public profile fields (no email or bio), `post_count`, and `usergroups`. Private users' `reputation` and `post_count` are null, and they sort last by reputation or posts.

- GET `/usergroups`: every usergroup's display details (`id`, `name`, `effect`, `color`, `icon_url`), used for the usergroup filter. No login required.
