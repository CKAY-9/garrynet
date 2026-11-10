# Permissions

Part of [GarryNet Design](DESIGN.md).

Permissions are managed through usergroups. Each usergroup has a bit flag field that stores its permissions.

## Usergroup

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

The two default usergroups are `user` and `admin`. 
- Admin: has all permissions
- User: all basic permissions and can access site

Members are added and removed from the group's page under `/admin/usergroups` (see [Managing Usergroups](#managing-usergroups)), or with the interactive shell on the backend with the command `usergroup add [USER_ID] [GROUP_NAME/ID]`.

To remove: `usergroup remove [USER_ID] [GROUP_NAME/ID]`

`USER_ID` also accepts a SteamID64. Other shell commands: `usergroup list`, `usergroup show [GROUP_NAME/ID]`, `user show [USER_ID]`, and `help`. Membership changes are logged.

The same commands can be run one-shot without attaching to the running server's terminal, e.g. `backend usergroup add 1 admin` (needs only `DATABASE_URL`).

The default usergroups are created on startup if no group with that name exists; existing groups are never overwritten. New users join `user` on their first login. Because they're matched by name, the default groups can't be renamed (only their case can change) or deleted.

## Managing Usergroups

Usergroups are created, edited and deleted from `/admin/usergroups` (see [ADMIN.md](ADMIN.md)). Every endpoint below requires `Admin Access` and `Usergroup Management`.

- GET `/admin/usergroups`: every usergroup in full.
- GET `/admin/usergroups/{id}`: one usergroup in full.
- POST `/admin/usergroups`: `{ name, effect, color, permissions, icon_url? }`; returns the new group (201). 409 if the name is taken.
- PUT `/admin/usergroups/{id}`: same body; replaces the group's details and bumps `last_updated`.
- DELETE `/admin/usergroups/{id}`: deletes the group; its members leave it (204).
- GET `/admin/usergroups/{id}/members`: the group's members, `{ id, username, avatar_url, added_at }[]`, by name.
- POST `/admin/usergroups/{id}/members`: `{ user_id }`; adds the user and returns the member (201). 404 if the user doesn't exist, 409 if they're already a member.
- DELETE `/admin/usergroups/{id}/members/{user_id}`: removes the user (204). 404 if they aren't a member.

Rules:
- `name`: 1-32 characters, unique regardless of case, not only digits (the shell takes a group id or name).
- `color`: hex (`#rgb`, `#rrggbb` or `#rrggbbaa`). `icon_url`: an http(s) URL or blank.
- `permissions`: only documented bits. Editors can only grant or revoke permissions they hold themselves (403 otherwise), so Usergroup Management can't be used to escalate.
- Adding or removing members requires holding every permission the group grants (403 otherwise), so a moderator can't put anyone, themselves included, into `admin`.
- A change that would remove the editor's own `Admin Access` or `Usergroup Management` (editing or deleting one of their groups, or leaving one) is refused with 400.
- Changes are logged.

## Permission Bits

- 0x1, Site Access: If clear, acts as a global ban. Users can only view and interact with support pages.
- 0x2, User Posts: If set, users can post on the forums.
- 0x4, Thread Management: If set, users can delete, lock, sticky, and edit forum threads and posts.
- 0x8, Category Management: If set, users can delete, create, edit, move, and configure forum categories.
- 0x10, Section Management: If set, users can delete, create, edit, move, and configure forum sections.
- 0x20, User Management: If set, users can ban, remove, suspend, and manage users.
- 0x40, Admin Access: If set, reveals admin navigation options and permits access to protected admin routes (e.g., `/admin`). Allows editing the loading screen and other sub-components.
- 0x80, Elevated Control: If set, users can perform **dangerous actions** across the whole site.
- 0x100, Usergroup Management: If set, users can create, edit, and remove usergroups, and add or remove users from them.
- 0x200, User Messages: If set, users can privately message other users.
- 0x400, Server Management: If set, users can create, edit, remove, and link/unlink Garry's Mod servers.
- 0x800, Community Management: If set, users can edit community details.
- 0x1000, Support Team: If set, users can access other tickets and help
- 0x2000, Store Management: If set, users can edit, add, and archive stores, items, and discounts. Cannot edit a store's grantable usergroups (requires `Elevated Control`). Accessing `/admin/store` also requires `Admin Access`.
- 0x4000, Store Finance: If set, users can view receipts and revenue, and issue refunds.

**Dangerous Actions**

Actions such as clearing all site data or triggering a panic (shutting down all services).

**Users in Multiple Groups**

A user's effective permissions are the combination (bitwise OR) of the permissions of all their usergroups.
