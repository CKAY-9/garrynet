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

To upgrade a user to a specific usergroup, use the interactive shell on the backend with the command `usergroup add [USER_ID] [GROUP_NAME/ID]`.

To remove: `usergroup remove [USER_ID] [GROUP_NAME/ID]`

`USER_ID` also accepts a SteamID64. Other shell commands: `usergroup list`, `usergroup show [GROUP_NAME/ID]`, `user show [USER_ID]`, and `help`. Membership changes are logged.

The same commands can be run one-shot without attaching to the running server's terminal, e.g. `backend usergroup add 1 admin` (needs only `DATABASE_URL`).

The default usergroups are created on startup if no group with that name exists; existing groups are never overwritten. New users join `user` on their first login.

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
