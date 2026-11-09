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

## /admin/usergroups

Admins can create, edit, remove usergroups.
Requires: Usergroup Management

## /admin/community

Admins can change the style of the loading screen.
Requires: Community Management