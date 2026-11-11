# Legal Documents

Part of [GarryNet Design](README.md).

GarryNet logs users in with Steam, stores player data and takes payments, so every deployment needs terms of service, a privacy policy and a refund policy. Each community writes its own. GarryNet ships templates to start from.

> [!IMPORTANT]
> The templates are a starting point, not legal advice. They describe what GarryNet itself stores. They don't cover anything you add (analytics, other addons, a Discord bot) or the laws that apply to you and your players.

## Documents

| Slug | Default title | Page |
| --- | --- | --- |
| `terms` | Terms of Service | `/legal/terms` |
| `privacy` | Privacy Policy | `/legal/privacy` |
| `refunds` | Refund Policy | `/legal/refunds` |

The set is fixed: the slugs are listed in `backend/src/models/legal.rs` (`TEMPLATES`), the migration's slug CHECK and `frontend/src/lib/legalPages.ts`.

```
slug: "terms" | "privacy" | "refunds"
title: string
body: string // Markdown
last_updated?: utc timestamp // null while still the default template
updated_by?: number // user id
```

## Templates and Storage

- The templates live in `backend/legal/*.md` and are compiled into the backend.
- On startup the backend inserts the template of any document that doesn't exist yet. It never overwrites an existing document, so edits survive restarts and upgrades. Improvements to the templates in a new release only reach new deployments, or admins who load the template again.
- From then on the database copy is what the site shows.
- The templates have `[square bracket]` placeholders (community name, contact email, ages, refund window) and a note at the top asking the admin to fill them in and delete the note.
- Bodies are Markdown, sanitized like forum posts: raw HTML is escaped, and links may only use http(s), mailto or relative URLs.

## Routes

- GET `/legal`: every document's `slug`, `title` and `last_updated`, in the order above (public)
- GET `/legal/{slug}`: the document (public; 404 for unknown slugs)
- GET `/legal/{slug}/template`: the shipped template (`slug`, `title`, `body`), requires Admin Access and Community Management
- PUT `/legal/{slug}`: replace `title` (at most 100 characters) and `body` (Markdown, at most 100,000 characters), requires Admin Access and Community Management. Sets `last_updated` and `updated_by`.

## Display

- `/legal/{slug}`: the title, "Last updated" (hidden while the document is still the template) and the rendered Markdown.
- Navigation: links to all three at the bottom of the sidebar, on every page.
- Login button: "By logging in you agree to our Terms and Privacy Policy."
- Store buy button: "By buying you agree to our Terms and Refund Policy."

Acceptance isn't recorded per user, and changed terms aren't pushed to users.

## Admin

See [ADMIN.md](ADMIN.md#adminlegal).
