# Garry's Mod Servers and Loading Screen

Part of [GarryNet Design](DESIGN.md). Player records (UserPlayer) are defined in [USERS.md](USERS.md).

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

Both styles fetch the user's tracked data for the particular server and display the relevant information. See [USER_STORIES.md](USER_STORIES.md) for details.

Backgrounds, animations, and structure are shared across servers. Icons, accent color, server and user/player details change per server.

Displayed information:
- Username/In-game name
- Avatar
- Server Name
- Last Logon / First Join
- Minimal Garry's Mod Progress Bar (with status text)

`/loading-screen` is the loading screen itself: full screen, with no site navigation. The progress bar follows the JavaScript callbacks Garry's Mod calls (`GameDetails`, `SetFilesTotal`, `SetFilesNeeded`, `DownloadingFile`, `SetStatusChanged`). Opened without query parameters, it previews the configured style with a fake player and server.

Point each server at its loading screen in `server.cfg`; Garry's Mod fills in `%s` (SteamID64) and `%m` (map):

```
sv_loadingurl "https://<site>/loading-screen?server=<SERVER_ID>&steamid=%s&mapname=%m"
```

### Routes

- GET `/loading-screen`: return the loading screen config
- PUT `/loading-screen`: set `{ style }`, requires Admin Access and Server Management
- GET `/servers/{id}/loading-screen?steam_id=`: everything the loading screen shows: `style`, `server` (never its `access_key`) and `player`. `player` combines the GarryNet user (name, avatar), their UserPlayer record for this server (`null` on a first visit) and, for players without an account, their Steam name and avatar. Public, since the game client isn't logged in.

## Loading Screen Config

One config for the whole community (always a single row).

```
id: number
style: "aero" | "sleek" // default "aero"
```

## Aero

Recreates the Frutiger Aero look while keeping the feel of a classic Garry's Mod loading screen.

## Sleek

A minimalist, clean, dim loading screen.
