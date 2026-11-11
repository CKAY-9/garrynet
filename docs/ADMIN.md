# Admin Pages

Part of [GarryNet Design](README.md).

## /admin: Dashboard

Admins should be able to see some basic statistics:
- registered users
- active users (users with a status of ONLINE)
- forum activity (posts over the past week)
- store activity (purchases and income over the past week)

There should be a navigation bar within the dashboard where they can navigate to:
- `/admin/usergroups`
- `/admin/servers`
- `/admin/loading-screen`
- `/admin/community`
- `/admin/legal`
- `/admin/forum`
- `/admin/store`

Every admin page requires Admin Access, plus the permission listed under each page below. The navigation only shows the pages the user can open.

## /admin/loading-screen

Admins can change the style of the loading screen.
Requires: Server Management

Style select:
- Aero
- Sleek

There should be a preview of the loading screen, constructed from the same components of the actual one. Use fake user/game data.

## /admin/servers

Admins can register, edit and delete Garry's Mod servers (see [SERVERS.md](SERVERS.md)).
Requires: Server Management

- The list shows each server's icon, accent color, gamemode, map, address and status.
- Registering takes a name and description, then shows the access key once, with a copy button.
- `/admin/servers/{id}` edits the details (name, description, icon, accent color, connection address, splash image), regenerates the access key (after confirming), shows the server's `sv_loadingurl` line and deletes the server (after typing its name).

## /admin/usergroups

Admins can create, edit, remove usergroups (see [PERMISSIONS.md](PERMISSIONS.md)).
Requires: Admin Access and Usergroup Management

- Lists every usergroup with its members count and permissions; creating one takes a name, color, effect, icon and permissions, then opens its page.
- `/admin/usergroups/{id}` edits the group (with a live chip preview) and deletes it (after typing its name).
- Only permissions the editor holds can be toggled. The default `user` and `admin` groups can't be renamed or deleted.
- The Members section lists the group's members (avatar, name, date added) with a filter by name or id. "Add user" searches users by name or id (10 results) and adds the one clicked; the "×" beside a member removes them.
- Members can only be changed by an editor holding every permission the group grants; others see the list read-only.

## /admin/community

Admins can edit the community details: name, description, logo, splash background, external links and key figures (add, remove, reorder). Changes apply when saved.
Requires: Community Management

**Game servers:** "Keep banned users off game servers" (off by default, `ban_from_servers` on `PUT /community`) makes linked servers refuse players banned on GarryNet and kick ones already playing. See [ADDON.md](ADDON.md#ban-enforcement).

## /admin/legal

Admins can edit the terms of service, privacy policy and refund policy (see [LEGAL.md](LEGAL.md)). There's a tab per document, each with a title and a Markdown body with a Write/Preview toggle. Each document saves on its own; unsaved edits are kept while switching tabs and marked with a dot.
Requires: Community Management

- While a document is still GarryNet's default template, a notice asks the admin to fill in the placeholders and review it.
- "Load default template" puts the shipped template in the editor. Nothing changes until it's saved.

## Banning and Suspending

Not an admin page: staff with Admin Access and User Management ban, suspend and lift those from the user's profile. See [USERS.md](USERS.md#banning-and-suspending).

## /admin/forum

Admins can create and delete forum sections and categories (see [FORUM.md](FORUM.md)). Deleting asks for a second click, since it removes every thread inside.
Requires: Section Management or Category Management. Each form only shows with its own permission.

- Lists every section with its categories, positions and activity.
- New section: name, accent color, description, icon URL, position.
- New category: section, name, description, position, locked.

## /admin/store

Admins can manage item stores (see [STORE.md](STORE.md)).
Requires: Store Management or Store Finance (and Admin Access, like every admin page). Management forms only show with Store Management; the receipts link only with Store Finance.

- Lists every store (enabled or not) with its server, currency, grantable usergroups and item count; creating one picks a server without a store.
- `/admin/store/{id}`: the store's settings (name, currency, message, enabled), its grantable usergroups (editable with Elevated Control), its items and its store-wide discounts.
- `/admin/store/{id}/items/new` and `/admin/store/{id}/items/{id}`: the item editor (details, pricing, stock, limits, rewards) and archiving.
- `/admin/store/receipts`: every receipt with buyer, item, price, status and delivery, filterable by status, store and buyer, with full refunds. Requires Store Finance.
