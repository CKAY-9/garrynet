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

Backgrounds, animations, and structure are shared across servers. Icons, accent color, and server and user details change per server.

## Loading Screen Config

```
id: number
style: "aero" | "sleek"
```

## Aero

Recreates the Frutiger Aero look while keeping the feel of a classic Garry's Mod loading screen.

## Sleek

A minimalist, clean, dim loading screen.
