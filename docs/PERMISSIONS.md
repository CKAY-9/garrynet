# Permissions

Part of [GarryNet Design](README.md).

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

`USER_ID` also accepts a SteamID64. Other shell commands: `usergroup list`, `usergroup show [GROUP_NAME/ID]`, `user show [USER_ID]`, `user logout [USER_ID]` (signs them out everywhere; see [AUTH.md](AUTH.md)), and `help`. Membership changes are logged.

The same commands can be run one-shot without attaching to the running server's terminal, e.g. `backend usergroup add 1 admin` (needs only `DATABASE_URL`).

The default usergroups are created on startup if no group with that name exists; existing groups are never overwritten. New users join `user` on their first login. Because they're matched by name, the default groups can't be renamed (only their case can change) or deleted.

## Managing Usergroups

Usergroups are created, edited and deleted from `/admin/usergroups` (see [ADMIN.md](ADMIN.md)). Every endpoint below requires `Admin Access` and `Usergroup Management`.

- GET `/admin/usergroups`: every usergroup in full.
- GET `/admin/usergroups/{id}`: one usergroup in full.
- POST `/admin/usergroups`: `{ name, effect, color, permissions, icon_url? }`; returns the new group (201). 409 if the name is taken.
- PUT `/admin/usergroups/{id}`: same body; replaces the group's details and bumps `last_updated`.
- DELETE `/admin/usergroups/{id}`: deletes the group; its members leave it (204). Requires holding every permission the group grants (403 otherwise), since deleting it revokes them.
- GET `/admin/usergroups/{id}/members`: the group's members, `{ id, username, avatar_url, added_at }[]`, by name.
- POST `/admin/usergroups/{id}/members`: `{ user_id }`; adds the user and returns the member (201). 404 if the user doesn't exist, 409 if they're already a member.
- DELETE `/admin/usergroups/{id}/members/{user_id}`: removes the user (204). 404 if they aren't a member.

Rules:
- `name`: 1-32 characters, unique regardless of case, not only digits (the shell takes a group id or name).
- `color`: hex (`#rgb`, `#rrggbb` or `#rrggbbaa`). `icon_url`: an http(s) URL or blank.
- `permissions`: only documented bits. Editors can only grant or revoke permissions they hold themselves (403 otherwise), so Usergroup Management can't be used to escalate.
- Adding or removing members requires holding every permission the group grants (403 otherwise), so a moderator can't put anyone, themselves included, into `admin`.
- Removing someone else also requires holding every permission they hold (403 otherwise), so nobody can demote a user above them.
- The `admin` group must keep at least one member (400 for removing the last one). The shell isn't held to this, so it can still be used to recover.
- A change that would remove the editor's own `Admin Access` or `Usergroup Management` (editing or deleting one of their groups, or leaving one) is refused with 400.
- Changes are logged.

## Permission Bits

- 0x1, Site Access: If clear, acts as a global ban. Users can only view and interact with support pages. The backend refuses what a ban forbids (posting, buying, uploading); the frontend's proxy (`frontend/src/proxy.ts`) redirects a banned user's other page requests to `/support`, leaving `/support`, `/notifications` and their receipts (`/store/purchases`) open. That redirect is only a courtesy: the pages it hides are public to logged-out visitors.
- 0x2, User Posts: If set, users can post on the forums.
- 0x4, Thread Management: If set, users can delete, lock, sticky, and edit forum threads and posts.
- 0x8, Category Management: If set, users can delete, create, edit, move, and configure forum categories.
- 0x10, Section Management: If set, users can delete, create, edit, move, and configure forum sections.
- 0x20, User Management: If set, users can manage users. Today this covers deleting other users' uploaded files; banning and suspending are **Planned** (see [User Punishment](USERS.md#user-punishment)).
- 0x40, Admin Access: If set, reveals admin navigation options and permits access to protected admin routes (e.g., `/admin`). Allows editing the loading screen and other sub-components.
- 0x80, Elevated Control: If set, users can perform **dangerous actions** across the whole site. Today this covers editing a store's grantable usergroups.
- 0x100, Usergroup Management: If set, users can create, edit, and remove usergroups, and add or remove users from them.
- 0x200, User Messages: If set, users can privately message other users. **Planned**: reserved for [Private Messages](README.md#private-messages), which aren't implemented.
- 0x400, Server Management: If set, users can create, edit, remove, and link/unlink Garry's Mod servers.
- 0x800, Community Management: If set, users can edit community details.
- 0x1000, Support Team: If set, users can access other users' tickets and help. Only together with Site Access: a banned team member is a regular user in support.
- 0x2000, Store Management: If set, users can edit, add, and archive stores, items, and discounts. Cannot edit a store's grantable usergroups (requires `Elevated Control`). Accessing `/admin/store` also requires `Admin Access`.
- 0x4000, Store Finance: If set, users can view receipts and revenue, and issue refunds.

**Dangerous Actions**

Actions that can't be undone or that affect the whole site, such as choosing which usergroups the store may grant. Clearing all site data or triggering a panic (shutting down all services) are **Planned**.

**Users in Multiple Groups**

A user's effective permissions are the combination (bitwise OR) of the permissions of all their usergroups.
