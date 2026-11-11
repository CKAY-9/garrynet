# Authentication

Part of [GarryNet Design](README.md).

All authentication uses JWT. A standard user JWT expires after 24 hours and is stored in an httpOnly cookie.

Servers authenticate with a rolling JWT: a token is valid for 30 days, and once it is an hour old the next heartbeat returns a replacement. Linking, unlinking and regenerating the access key revoke all of a server's earlier tokens. See [ADDON.md](ADDON.md#server-tokens).

Users log in with Steam. If no GarryNet account exists for the Steam account, one is created, and the user is granted access to it via JWT.

## Banned Users

Users whose usergroups clear `Site Access` can only view and use the support pages, so they can still appeal. They keep receiving notifications.

**Planned:** users with an active ban (see [User Punishment](USERS.md#user-punishment)) will be treated the same way, without changing their usergroups.
