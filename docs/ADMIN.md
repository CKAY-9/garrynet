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