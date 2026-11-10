# Admin Pages

Part of [GarryNet Design](DESIGN.md).

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
- `/admin/forum`
- `/admin/store`

They must have the correct permission to view/access the page.

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

Admins can create, edit, remove usergroups.
Requires: Usergroup Management

## /admin/community

Admins can edit the community details: name, description, logo, splash background, external links and key figures (add, remove, reorder). Changes apply when saved.
Requires: Community Management

## /admin/forum

Admins can create and delete forum sections and categories (see [FORUM.md](FORUM.md)). Deleting asks for a second click, since it removes every thread inside.
Requires: Section Management or Category Management. Each form only shows with its own permission.

- Lists every section with its categories, positions and activity.
- New section: name, accent color, description, icon URL, position.
- New category: section, name, description, position, locked.

## /admin/store

Admins can manage item stores (see [STORE.md](STORE.md)).
Requires: Store Management or Store Finance. Management forms only show with Store Management; the receipts link only with Store Finance.

- Lists every store (enabled or not) with its server, currency, grantable usergroups and item count; creating one picks a server without a store.
- `/admin/store/{id}`: the store's settings (name, currency, message, enabled), its grantable usergroups (editable with Elevated Control), its items and its store-wide discounts.
- `/admin/store/{id}/items/new` and `/admin/store/{id}/items/{id}`: the item editor (details, pricing, stock, limits, rewards) and archiving.
- `/admin/store/receipts`: every receipt with buyer, item, price, status and delivery, filterable by status, store and buyer, with full refunds. Requires Store Finance.
