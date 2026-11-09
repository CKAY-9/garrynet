# Authentication

Part of [GarryNet Design](DESIGN.md).

All authentication uses JWT. A standard user JWT expires after 24 hours and is stored in an httpOnly cookie.

Servers authenticate with a rolling JWT: a new token is issued every few requests for security.

Users log in with Steam. If no GarryNet account exists for the Steam account, one is created, and the user is granted access to it via JWT.

## Banned Users

Users with an active ban (see User Punishment in [USERS.md](USERS.md)) cannot post or interact with anyone except administration.

Users whose usergroups clear `Site Access` can only view support pages.
