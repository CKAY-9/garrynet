# Addon Details

Part of [GarryNet Design](DESIGN.md).

The GarryNet Garry's Mod addon is responisble for communicating with GarryNet. It will update player data, fetch unrealized store requests, config changes, etc.

## !gnet

A superadmin should be able to execute `!gnet` in chat and be presented with a dark-blue menu, with a darkened and blurred background.

**Not Linked**
Prompt the superadmin for the generated server access key. It will be sent to `/gnet/server/link` with it as an authorization header. Exchanging should result in a JWT for the server that it can use to authenticate.

**Linked**
Display options for unlinking (revoking/removing JWT and access key, if it still exists), force interval update

## Intervals

A GNet interval should occur every minute and sync player information to GarryNet. This is done by HTTP requests to `/gnet/server/{SERVER_ID}/[route]` on the backend.