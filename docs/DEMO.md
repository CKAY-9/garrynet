# Demo Mode

Part of [GravNet Design](README.md).

A public demo lets anyone log in and try every feature without real payments, and without what they post piling up. Demo mode is turned on with `DEMO_MODE=true` in the backend's environment. `docker-compose.demo.yml` sets it, on top of the regular Docker deployment (see [Deploying](#deploying)).

## Behavior

- **Stripe is test-only.** The backend refuses to start if `STRIPE_SECRET_KEY` isn't a test key (`sk_test_...` or `rk_test_...`), and the webhook acknowledges but ignores live-mode events. Without Stripe keys the store can be browsed but nothing can be bought, as usual.
- **Visitors' content is cleared daily.** At midnight UTC, everything users without Admin Access created is deleted (see [Reset](#reset)). Their accounts are kept.
- **The site says so.** Every page shows a banner with the next reset time, and the buy button shows Stripe's test card (`4242 4242 4242 4242`, any future expiry, any CVC).

Admins are users in a usergroup with **Admin Access**, whatever punishments they're under. Everyone else counts as a visitor. Admin pages already need Admin Access to change anything, so visitors can't change the community, servers, stores, usergroups or legal pages.

## Reset

`backend/src/demo.rs`. Runs at midnight UTC (never on startup, so restarts don't clear anything), and on demand with the `demo reset` shell command. Refuses to run unless demo mode is on.

First, visitors' open Checkout Sessions are expired, so none can be paid after its purchase is gone. Then, in one transaction:

| What | Reset |
|---|---|
| Forum | Visitors' threads, with every reply in them (admins' replies included). Visitors' replies in admins' threads; those threads' last activity moves back to their newest remaining post. Visitors' likes and dislikes. |
| Support | Visitors' tickets, with every message in them. Visitors' messages in admins' tickets. Visitors are removed from tickets' associated users and responders. FAQs created by visitors, and visitors in FAQs' editors. |
| Private messages | Every message sent or received by a visitor. |
| Store | Visitors' purchases, receipts and deliveries. The stock their pending and paid purchases held is returned, and their paid purchases come off each item's purchase count. |
| Files | Visitors' uploads, and every file attached to a post, support message or private message the reset deletes. Community files (logo, splash) are kept. |
| Notifications | All of them: most point at what was just deleted. |

Kept: users' accounts, profiles (bio, private profile), usergroup memberships, sessions, punishments and in-game player records; and everything admins created.

After the transaction commits, the deleted files' bytes are removed from storage.

Limitations:

- Rewards already delivered in-game for a visitor's test purchase stay in-game; the reset only deletes the delivery record.
- Changes that leave no record of who made them can't be undone: for example, if visitors are given a usergroup with Thread Management, a thread they lock stays locked. With the default `user` group this can't happen.

## Routes

- GET `/demo`: `{ enabled: boolean, next_reset: utc timestamp | null }`. Public. `next_reset` is null unless `enabled`.

## Deploying

Run the demo from its own clone of the repository: its `.env` files and ports 80/443 can't be shared with another deployment on the same machine.

1. Point DNS records for the site and API domains (e.g. `gnet.example.com` and `api.gnet.example.com`) at the machine, and open ports 80 and 443.
2. Copy `.env.example` to `.env` in the repository root and set:
    ```sh
    SITE_DOMAIN=gnet.example.com
    API_DOMAIN=api.gnet.example.com
    ```
    The database password is generated on first start (see the README's Docker setup).
3. Copy `backend/.env.example` to `backend/.env`:
    ```sh
    BACKEND_URL=https://api.gnet.example.com
    FRONTEND_URL=https://gnet.example.com
    COOKIE_DOMAIN=gnet.example.com
    JWT_SECRET=<openssl rand -base64 48>
    STEAM_API_KEY=<your key>
    STRIPE_SECRET_KEY=sk_test_...
    STRIPE_WEBHOOK_SECRET=whsec_...
    ```
    `DEMO_MODE` is set by `docker-compose.demo.yml`. Create the webhook endpoint `https://api.gnet.example.com/stripe/webhook` in the Stripe dashboard **in test mode**, with the events listed in the [main README](../README.md#store-payments-stripe).
4. Copy `frontend/.env.example` to `frontend/.env` and set `NEXT_PUBLIC_API_URL=https://api.gnet.example.com`.
5. Start it:
    ```sh
    docker compose -f docker-compose.yml -f docker-compose.demo.yml up -d --build
    ```
    Caddy gets certificates for both domains on first start. Only Caddy publishes ports; the frontend and backend are reached through it. It also sets security headers and rate limits each IP, like the regular deployment ([README](../README.md#reverse-proxy)).
6. Log in with Steam, then make yourself an admin:
    ```sh
    docker compose -f docker-compose.yml -f docker-compose.demo.yml exec backend backend usergroup add <your SteamID64> admin
    ```

To clear visitors' content right away: `docker compose -f docker-compose.yml -f docker-compose.demo.yml exec backend backend demo reset`.

### Domains and cookies

The browser only calls the backend directly to log in and out (page navigations) and to upload files (allowed from `FRONTEND_URL`), so the backend needs no other CORS setup. The login cookie must reach both the frontend and the backend, so `COOKIE_DOMAIN` has to cover both domains. Put the API under the site's domain (`api.gnet.example.com` under `gnet.example.com`) so that `COOKIE_DOMAIN` can be the site's own domain. With sibling domains (`gnet.example.com` and `gnet-api.example.com`), it would have to be `example.com`, which sends the demo's login cookie to every site on `example.com`.
