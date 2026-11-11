# Authentication

Part of [GarryNet Design](README.md).

All authentication uses JWT. A standard user JWT expires after 24 hours and is stored in an httpOnly cookie.

The cookie is `SameSite=Lax`, which keeps it off cross-site POSTs but not off requests from sibling subdomains (they count as the same site). So a state-changing request (anything but GET, HEAD or OPTIONS) authenticated by the cookie is refused with 403 when its `Origin` isn't the frontend's or the backend's (from `FRONTEND_URL` and `BACKEND_URL`). Requests with `Authorization: Bearer` (the Next.js server, the addon) or without an `Origin` header (not from a browser page) aren't affected.

User tokens are revocable: each carries the user's `token_version`, and a token is only accepted while that still matches. Logging out (POST `/auth/logout`) bumps it, so it signs the user out on every device and a stolen token stops working too. The backend shell can do the same for any user with `user logout [USER_ID]`, e.g. after an account was compromised.

Servers authenticate with a rolling JWT: a token is valid for 30 days, and once it is an hour old the next heartbeat returns a replacement. Linking, unlinking and regenerating the access key revoke all of a server's earlier tokens. See [ADDON.md](ADDON.md#server-tokens).

Users log in with Steam. If no GarryNet account exists for the Steam account, one is created, and the user is granted access to it via JWT.

Each login is tied to the browser that started it: GET `/auth/steam` sets a short-lived (10 minutes) `garrynet_login_state` cookie holding a random nonce and puts the same nonce in the OpenID `return_to` (`/auth/steam/callback?state=...`), which Steam signs. The callback only accepts an assertion whose `return_to` carries the cookie's nonce, then clears the cookie. Otherwise someone could send a victim to the callback with an assertion for the attacker's own account and have the victim use it unknowingly (login CSRF). A login that takes longer than 10 minutes, or finishes in another browser, has to be started again.

## Banned Users

Users whose usergroups clear `Site Access` can only view and use the support pages, so they can still appeal. They keep receiving notifications.

**Planned:** users with an active ban (see [User Punishment](USERS.md#user-punishment)) will be treated the same way, without changing their usergroups.
