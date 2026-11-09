# GarryNet Design

## Users

Users are authenticated using Steam. Everyone logs in with their Steam account and GarryNet stores each user in its own table entry.

### User

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

### User Status

- Online: set when the user accesses a route/resource
- Idle: set after 5 minutes without access
- Offline: set after 10 minutes of being idle (15 minutes since last access)

```
UserStatus:
ONLINE,
IDLE,
OFFLINE
```

### User Punishment

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

### User Player

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
stats: json // gamemode-specific data, e.g. `darkrp_money`
time_played: number // in minutes; the addon updates it every minute
```

### Player Punishment

Player Punishments are in-game punishments such as kicks, bans, jails, and warnings.

To be implemented.

## Permissions

Permissions are managed through usergroups. Each usergroup has a bit flag field that stores its permissions.

### Usergroup

```
id: number
name: string
effect: "halo" | "rainbow" | "flash" | "none"
color: string // hex
members: number[] // ids of users who belong to this group
permissions: number // bit flag
icon_url?: string // can be null/empty
created_at: utc timestamp
last_updated?: utc timestamp // empty/null if no changes since creation
```

### Permission Bits

- 0x1, Site Access: If clear, acts as a global ban. Users can only view and interact with support pages.
- 0x2, User Posts: If set, users can post on the forums.
- 0x4, Thread Management: If set, users can delete, lock, sticky, and edit forum threads and posts.
- 0x8, Category Management: If set, users can delete, create, edit, move, and configure forum categories.
- 0x10, Section Management: If set, users can delete, create, edit, move, and configure forum sections.
- 0x20, User Management: If set, users can ban, remove, suspend, and manage users.
- 0x40, Admin Access: If set, reveals admin navigation options and permits access to protected admin routes (e.g., `/admin`). Allows editing the loading screen and other components.
- 0x80, Elevated Control: If set, users can perform **dangerous actions** across the whole site.
- 0x100, Usergroup Control: If set, users can create, edit, and remove usergroups, and add or remove users from them.
- 0x200, User Messages: If set, users can privately message other users.
- 0x400, Server Management: If set, users can create, edit, remove, and link/unlink Garry's Mod servers.

**Dangerous Actions**

Actions such as clearing all site data or triggering a panic (shutting down all services).

**Users in Multiple Groups**

A user's effective permissions are the combination (bitwise OR) of the permissions of all their usergroups.

## Forum

The forum is the community hub, where users can interact and post about any topic. The forum holds sections, sections hold categories, categories hold threads, and threads contain posts.

### Section

```
id: number
name: string
accent: string // hex color
icon_url?: string // can be null/empty
description: string // max 256 characters
position: number // listed from highest to lowest; -1 = always last
created_at: utc timestamp
last_updated?: utc timestamp // empty/null if no changes since creation
```

### Category

```
id: number
name: string
section_id: number
description: string // max 256 characters
locked: boolean // prevents new threads from being created in it
position: number // same ordering rules as Section
created_at: utc timestamp
last_updated?: utc timestamp // empty/null if no changes since creation
```

### Thread

```
id: number
author_id: number
category_id: number
title: string
thread_post_id: number // id of the thread's first post
created_at: utc timestamp
last_activity: utc timestamp
locked: boolean // prevents any changes to the thread
sticky: boolean // lists the thread first, alongside other stickies
allow_template: boolean // if set, users can copy this thread as a template for new threads
```

### Post

```
id: number
author_id: number
thread_id: number
content: string
created_at: utc timestamp
edited?: utc timestamp // null/empty if no edit applied
```

## Garry's Mod Server

Servers are first registered on the website, then connected by pasting the generated `access_key` into the GarryNet addon menu. This lets the server communicate with GarryNet. After the first connection, the access key is revoked and replaced with a rolling token. If the server and GarryNet fall out of sync, regenerate the `access_key`.

```
id: number
name: string
description: string
gamemode: string // "" until the first heartbeat
splash_url?: string // shown on the website, not on the loading screen
icon_url?: string
color: string // hex
connection_address: string
map: string // "" until the first heartbeat
access_key: string // used to verify the server
last_heartbeat: utc timestamp // when the server last checked in
linked: boolean // true once the server has connected with its access key
created_at: utc timestamp
```

## Loading Screen

The loading screen is meant for Garry's Mod servers using its standard implementation. It has two styles:
1. Aero
2. Sleek

Both styles fetch the user's tracked data for the particular server and display the relevant information. See User Stories for details.

Backgrounds, animations, and structure are shared across servers. Icons, accent color, and server and user details change per server.

### Loading Screen Config

```
id: number
style: "aero" | "sleek"
```

### Aero

Recreates the Frutiger Aero look while keeping the feel of a classic Garry's Mod loading screen.

### Sleek

A minimalist, clean, dim loading screen.

## Authentication

All authentication uses JWT. A standard user JWT expires after 24 hours and is stored in an httpOnly cookie.

Servers authenticate with a rolling JWT: a new token is issued every few requests for security.

Users log in with Steam. If no GarryNet account exists for the Steam account, one is created, and the user is granted access to it via JWT.

### Banned Users

Users with an active ban (see User Punishment) cannot post or interact with anyone except administration.

Users whose usergroups clear `Site Access` can only view support pages.

## Admin Pages

### /admin: Dashboard

Admins should be able to see some basic statistics:
- registered users
- active users (users with a status of ONLINE)
- forum activity (posts over the past week)
- store activity (purchases and income over the past week)

### /admin/loading-screen

Admins can change the style of the loading screen.

## Store

To be implemented. Will use Stripe.

## Support Pages

To be implemented. Will include support tickets and documentation.

## Private Messages

To be implemented.

## User Stories

### General

**Splash Page**

1. As a general user, I should see the side navigation and the community's splash page.
   - The splash page shows the community name, description, background, and general information.
2. I should be able to navigate to `/forums`, `/store`, `/players`, and `/loading-screen`, and see a "Login with Steam" button.

**Viewing Profile**

1. As a logged-in user, I should see my profile in place of the "Login with Steam" button, so that I can navigate to it.
2. Navigating to my profile should redirect to `/players/{STEAM_ID}`, where `STEAM_ID` is my own.

**Admin View**

1. As a logged-in user with the `Admin Access` permission, I should see the `/admin` navigation item.
2. Clicking it should take me to `/admin` without being redirected away.

### User Login

**Regular Flow**

1. As a logged-out user, I want to click the "Login with Steam" button, so that I am redirected to Steam's login flow.
2. Steam redirects to the backend (`/auth/steam/callback`), which signs the user in with a JWT.
3. The user is redirected to the frontend. Next.js can read the JWT server-side and use it to fetch the user's data.

### Loading Screen

**Loading into Garry's Mod server**

1. As a player, I want the loading screen to fetch my player information for the server, so that I can see gamemode attributes, my last logon, and my Steam details.

**Viewing on `/loading-screen`**

1. As a general user, I want the loading screen to show a fake user and fake gamemode/server details, so that I can preview it.

### Garry's Mod Servers

**Registering a Garry's Mod Server**
1. As a user with `Server Management`, I want to register a new server, so that I can link it later.
2. I first provide a name and description.
3. I am given a generated access key to use on a server running the GarryNet addon.
4. I paste the key into the GarryNet addon menu to link the server.

## Site Design

The whole site should have a consistent structure of [SIDE NAVIGATION] [CONTENT].

### Themes

#### Dark

The default theme: dark and cool, easy on the eyes.

#### Light

A toggle switches to light mode, a warmer, brighter theme.

## Architecture

### Frontend

Next.js with TypeScript.

### Backend

Actix Web (Rust) with PostgreSQL.

### Addon

Standard GLua addon for Garry's Mod.