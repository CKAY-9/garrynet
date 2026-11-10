# GarryNet Design

GarryNet is a community platform (forum, store, Garry's Mod server integration) with one deployment per community.

## Documents

- [USERS.md](USERS.md): users, status, punishments, per-server players
- [PERMISSIONS.md](PERMISSIONS.md): usergroups and permission bits
- [FORUM.md](FORUM.md): sections, categories, threads, posts
- [SUPPORT.MD](SUPPORT.MD): support tickets, messages, FAQs
- [STORE.md](STORE.md): item stores, items, purchases, receipts, fulfillment
- [STORAGE.md](STORAGE.md): file uploads, local and S3-compatible storage
- [SERVERS.md](SERVERS.md): Garry's Mod servers and the loading screen
- [ADDON.md](ADDON.md): the Garry's Mod addon
- [AUTH.md](AUTH.md): authentication and banned users
- [ADMIN.md](ADMIN.md): admin pages
- [USER_STORIES.md](USER_STORIES.md): user stories
- [SITE_DESIGN.md](SITE_DESIGN.md): frontend layout, navigation, themes
- [ARCHITECTURE.md](ARCHITECTURE.md): frontend, backend, addon stack

## Community

Each deployment of GarryNet is meant for one community.

```
id: number
name: string
description: string
logo_url?: string // http(s) URL
splash_background?: string // http(s) image URL, displayed behind the homepage header
external_links: string[] // http(s) URLs, default = []
created_at: utc timestamp
last_updated? utc timestamp
key_figures: json // array of { user_id?: number, name: string, profile_picture: string, role: string, description: string }
```

If starting for the first time, a skeleton community entry should be created that can be edited later. There is only ever one community; existing details are never overwritten on startup.

Skeleton Community:
- name: "GarryNet Community"
- description: "Set your own community description in the admin dashboard"
- key_figures: [
    {
        name: "CKAY9",
        profile_picture: "https://avatars.githubusercontent.com/u/53030585?v=4",
        role: "GarryNet Developer",
        description: "Hey, this is the thing I made."
    }
]

### Routes

- POST `/community`: create a new community, requires Admin Access and Community Management (creation shouldn't need to be called; 409 if one already exists)
- GET `/community`: return the current community object (public; recreates the skeleton if missing)
- PUT `/community`: replace the community details, requires Admin Access and Community Management

POST/PUT take `name`, `description`, `logo_url`, `splash_background`, `external_links`, `key_figures`. Blank optional URLs become null; a key figure's `user_id` must belong to an existing user.

### Display

- Homepage: splash background, logo, name, description, external links, then key figures (linking to their profile when `user_id` is set).
- Navigation: logo and name at the top, on every page. Without a logo, the name's initial is shown.

## Store

See [STORE.md](STORE.md). Uses Stripe Checkout; the addon delivers purchases in-game.

## Private Messages

To be implemented.
