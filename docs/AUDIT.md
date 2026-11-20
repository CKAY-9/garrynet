# Security and Store Flow Audit

A read-only review of the backend, done on 2026-11-18 at commit `00942b6`. Line references in the findings point at that commit. Unlike the other documents here, the findings are a snapshot; only the **Status** of each is updated as they're dealt with.

**Status as of commit `4d597c0`:** 14 fixed (#1, #2, #3, #4, #5, #6, #7, #9, #10, #13, #14, #16, #17, #19), 1 accepted as not an issue (#8), 4 open.

**Scope:** the backend (`backend/`): auth, permissions, the Stripe store flow, uploads and storage, Markdown handling, the addon API (`/gnet/server/*`) and the deployment config (Caddy, Compose). The addon's reward delivery was checked as part of the store flow. The frontend was out of scope.

**Severity:** **High** can lose money or misdeliver rewards on its own; **Medium** is exploitable or harmful in realistic conditions; **Low** is hardening, edge cases, or behavior that's already documented.

## Summary

| # | Area | Finding | Severity | Status |
|---|------|---------|----------|--------|
| 1 | Store | Refund/dispute events arriving before their receipt are dropped | High | Fixed (`e00cb4e`) |
| 2 | Store | DarkRP money isn't taken back on refund or chargeback | Medium | Fixed, opt-in (`4d597c0`) |
| 3 | Store | No way to redeliver after a dispute is won | Medium | Fixed (`e00cb4e`) |
| 4 | Store | Deleting a server drops paid, undelivered fulfillments | Medium | Fixed (`39d69b8`) |
| 5 | Store | Refunds are recorded before Stripe confirms them | Low–Medium | Fixed (`4d597c0`) |
| 6 | Store | Partial refunds from the Stripe dashboard are invisible | Low | Fixed (`4d597c0`) |
| 7 | Store | Receipt list capped at 500, no pagination | Low | Fixed (`e00cb4e`) |
| 8 | Store | Reinstated purchases skip stock and per-user limits | Low | Accepted |
| 9 | Store | Stripe IDs aren't namespaced per deployment | Low | Fixed (`e00cb4e`) |
| 10 | Security | No upload quota; S3 staging uploads are unbounded | Medium | Fixed (`4d597c0`) |
| 11 | Security | `COOKIE_DOMAIN` advice sends the login token to every subdomain | Medium | Open |
| 12 | Privacy | Loading screen exposes private users' player records | Medium | Open |
| 13 | Security | No HSTS header | Low–Medium | Fixed (`4d597c0`) |
| 14 | Abuse | No app-level rate limit on forum and support posting | Low–Medium | Fixed (`e00cb4e`) |
| 15 | Abuse | Unbounded or expensive public reads | Low | Open |
| 16 | Permissions | Users of equal rank can ban each other | Low | Fixed (`4d597c0`) |
| 17 | Permissions | Usergroup editors can restyle groups that outrank them | Low | Fixed (`4d597c0`) |
| 18 | Privacy | S3 private attachments readable at the public URL | Low | Open |
| 19 | Deployment | Postgres password defaults to `gravnet` | Low | Fixed (`4d597c0`) |

## Store Flow

### 1. Refund and dispute events that arrive before their receipt are dropped (High)

**Status:** Fixed in `e00cb4e`. A refund or dispute for a payment that isn't recorded yet is kept in `store_early_settlements` (a refund outranking a dispute) and applied by `Purchase::mark_paid` when the payment is recorded; both take an advisory lock per PaymentIntent, so neither can miss the other. Unclaimed entries are swept after 30 days. See STORE.md, "Refunds and Disputes".

**Where:** `settle` in [routes/store.rs:627-649](../backend/src/routes/store.rs#L627-L649).

When `charge.refunded` or `charge.dispute.created` names a payment intent with no receipt yet, `settle` logs "unknown payment intent" and returns `Ok`, so the webhook answers 200 and Stripe never redelivers the event. Stripe doesn't guarantee event order, and `checkout.session.completed` can also be stuck retrying after a database error. When the completion is finally processed, the purchase is marked PAID and its rewards delivered, even though the money has been returned.

**Suggested fix:** answer with a non-2xx status for an unknown payment intent so Stripe retries the event, or record the pending refund or dispute by payment intent and apply it in `Purchase::mark_paid`.

### 2. DarkRP money isn't taken back on refund or chargeback (Medium)

**Status:** Fixed in `4d597c0`, as an option. Each store has `money_on_revoke`: `KEEP` (the default, as before) leaves the money with the player; `DEDUCT` has the addon take back what was delivered, or the player's whole balance if that's less (never below 0), once they're online. The addon records each deduction before making it, so it's never made twice, and reports the amount, which reinstating a purchase (#3) delivers again. See STORE.md, "Refunds and Disputes".

**Where:** [sv_store_rewards.lua:22](../addon/lua/gravnet/sv_store_rewards.lua#L22), [:379-380](../addon/lua/gravnet/sv_store_rewards.lua#L379-L380).

The addon records usergroup and weapon grants so a revocation can take them back, but `darkrp_money` is applied with `addMoney` and never recorded. A buyer can buy money, spend it, then dispute the charge and keep it.

**Suggested fix:** take the money back on revocation (allowing a negative balance, or clamping at zero and logging the shortfall), or at least tell staff when a revoked fulfillment included money. Consider documenting that money items carry chargeback risk.

### 3. No way to redeliver after a dispute is won (Medium)

**Status:** Fixed in `e00cb4e`. `charge.dispute.closed` records the outcome on the receipt (`dispute_outcome`), and `POST /admin/store/receipts/{id}/reinstate` (Store Finance, with a button on `/admin/store/receipts`) moves a disputed receipt whose dispute was won (`won` or `warning_closed`) back to PAID and has the addon deliver its rewards again. Rewards that had been delivered come back without their DarkRP money (see #2). The buyer is notified.

**Where:** `charge.dispute.closed` in [routes/store.rs:516-525](../backend/src/routes/store.rs#L516-L525); `Fulfillment::retry` in [models/store.rs:1247-1257](../backend/src/models/store.rs#L1247-L1257); `Receipt::set_status` in [models/store.rs:1153-1156](../backend/src/models/store.rs#L1153-L1156).

A won dispute leaves the receipt DISPUTED and its fulfillment REVOKED, "until staff act". But staff have nothing to act with: `retry` only works on PAID receipts, and `set_status` never moves a receipt back to PAID. A buyer whose payment stood gets no way back to their rewards.

**Suggested fix:** an admin action (Store Finance) that moves a DISPUTED receipt back to PAID and creates or resets its fulfillment, or do this automatically on a won `charge.dispute.closed`.

### 4. Deleting a server drops paid, undelivered fulfillments (Medium)

**Status:** Fixed in `39d69b8`. `DELETE /servers/{id}` returns 400 while paid purchases from the server's store are undelivered, checked again under a lock on the server's row as it's deleted. Store Finance can refund them all at once with `POST /admin/store/servers/{id}/undelivered/refund`, from the server's admin page, which also disables its delete button meanwhile. Unapplied revocations don't block deletion.

**Where:** `delete_server` in [routes/servers.rs:95-109](../backend/src/routes/servers.rs#L95-L109); `store_fulfillments.server_id` in [create_store.sql:143](../backend/migrations/20261109000005_create_store.sql#L143).

`close_checkouts_for_server` only checks for open checkouts. Fulfillments reference the server with `ON DELETE CASCADE`, so PENDING and FAILED fulfillments of paid receipts disappear with it. The receipt survives with `server_id` NULL, so the buyer has paid, nothing will be delivered, and nobody is told.

**Suggested fix:** refuse to delete a server with undelivered fulfillments of paid receipts (as is done for unsettled checkouts), or require them to be refunded first.

### 5. Refunds are recorded before Stripe confirms them (Low–Medium)

**Status:** Fixed in `4d597c0`. Every refund event (`charge.refunded`, and the newly handled `charge.refund.updated`) has the backend read the payment's charges and refunds from Stripe, under the payment's lock, into `store_payment_refunds`. A refund that fails or is cancelled after the receipt was marked refunded flags it "Refund failed" on `/admin/store/receipts` (and logs an error); Store Finance can refund it again (with a new idempotency key) or reinstate it, which delivers the rewards again. A refund Stripe fails at once isn't recorded as a refund, and pending refunds are shown as such. The receipt is still marked refunded as soon as Stripe accepts the refund, without a pending-refund state. See STORE.md, "Refunds and Disputes".

**Where:** `refund` in [routes/store_admin.rs:380-387](../backend/src/routes/store_admin.rs#L380-L387).

The receipt is set to REFUNDED and its rewards revoked as soon as Stripe accepts the refund request, whatever the refund's `status` (`pending`, `requires_action`). `refund.failed` and `charge.refund.updated` aren't handled, so if the refund later fails the buyer has neither their money nor their rewards, and GravNet still shows it refunded.

**Suggested fix:** subscribe to `refund.failed` (or `charge.refund.updated`) and surface failed refunds to Store Finance; optionally hold the receipt in a pending-refund state until it succeeds.

### 6. Partial refunds from the Stripe dashboard are invisible (Low)

**Status:** Fixed in `4d597c0`, with #5: partial refunds are recorded per payment (`refunds` on staff receipt views, shown on `/admin/store/receipts`). Income (`store.income_week` and the receipts page total) counts each receipt's price in proportion to what wasn't refunded. Partially refunded receipts stay paid and keep their rewards.

**Where:** `charge.refunded` in [routes/store.rs:507-511](../backend/src/routes/store.rs#L507-L511).

Only full refunds (`refunded: true`) are acted on. A partial refund leaves the receipt PAID, so the admin dashboard's weekly income overstates revenue. Keeping the rewards may be intended, but nothing records the partial refund.

**Suggested fix:** record `amount_refunded` on the receipt and subtract it in `Receipt::income_since_days`.

### 7. Receipt list capped at 500, no pagination (Low)

**Status:** Fixed in `e00cb4e`. `GET /admin/store/receipts` takes `limit` (1 to 500, default 100) and `cursor`, and returns `{ items, next_cursor }`; the receipts page links to older pages and back.

**Where:** `MAX_RECEIPTS` in [routes/store_admin.rs:68](../backend/src/routes/store_admin.rs#L68).

Older receipts can only be reached through the buyer or store filters, so on a busy store they're hard to find and refund from the admin pages.

**Suggested fix:** cursor pagination, as notifications and private messages already have.

### 8. Reinstated purchases skip stock and per-user limits (Low)

**Status:** Accepted; not considered an issue. It needs a payment on a session GravNet had already cancelled, and the buyer has paid.

**Where:** `Purchase::mark_paid` in [models/store.rs:981-989](../backend/src/models/store.rs#L981-L989).

A cancelled purchase that turns out to have been paid is reinstated, taking its stock again but clamping at zero, and without rechecking `max_per_user`. Limited items can oversell. This is documented and rare (it needs a payment on a session GravNet had cancelled), but worth knowing.

### 9. Stripe IDs aren't namespaced per deployment (Low)

**Status:** Fixed in `e00cb4e`. Checkout Sessions and their PaymentIntents carry `gravnet_deployment` (the backend's host and port, from `BACKEND_URL`) in their metadata, and idempotency keys are prefixed with it. Checkout events for another deployment's sessions are ignored, and `metadata.purchase_id` is only trusted on this deployment's sessions. Changing `BACKEND_URL`'s host renames the deployment; the stale-purchase sweeper still records checkouts opened before the change.

**Where:** [stripe.rs:133-138](../backend/src/stripe.rs#L133-L138), [:156](../backend/src/stripe.rs#L156); [models/store.rs:951-957](../backend/src/models/store.rs#L951-L957).

The idempotency key is `purchase-{id}`, and the webhook can find a purchase by `metadata.purchase_id` when its session id wasn't recorded. When two deployments share one Stripe account (staging and production, or a demo on a shared test account), both receive every webhook, purchase ids collide, and in that narrow case one deployment can match the other's payment. Idempotency keys can also collide within Stripe's 24-hour window.

**Suggested fix:** add a deployment identifier (e.g. derived from `BACKEND_URL`) to the idempotency key and session metadata, and ignore sessions that don't carry it.

## Security and Abuse

### 10. No upload quota; S3 staging uploads are unbounded (Medium)

**Status:** Fixed in `4d597c0`. Each user may hold at most `STORAGE_USER_QUOTA` bytes (default 20 times `STORAGE_MAX_FILE_SIZE`) in files that aren't attached to anything, pending uploads included, checked under a lock per user. Users without Site Access get a burst of 3 uploads instead of 10. Presigned S3 upload URLs are signed with `Content-Length`, so the provider refuses any other size: `object_store` won't sign it, so the URL is signed again (checked against AWS's published example, `object_store`'s own signatures, and a local Garage, which refused bodies of the wrong size). Completion still checks the size, for providers that don't enforce signed headers. There's still no global quota across accounts; the `staging/` lifecycle rule remains a recommended backstop.

**Where:** `UPLOAD` in [routes/files.rs:42](../backend/src/routes/files.rs#L42), the rate limit at [:152-160](../backend/src/routes/files.rs#L152-L160); S3 presigning in [storage.rs:232-244](../backend/src/storage.rs#L232-L244); `UNATTACHED_FILE_HOURS` in [main.rs:234](../backend/src/main.rs#L234).

Any logged-in user can upload, banned users included (for support appeals). The rate limit allows 10 uploads per 30 seconds at the default 2 MiB, about 2.4 GB an hour per account, and unattached files are kept for 24 hours. There's no per-user or global quota, and Steam accounts cost nothing to create. With S3 it's worse: a presigned PUT can't limit size, so each upload target accepts up to 5 GB in `staging/` until the sweep (or completion) removes it, and the URL keeps accepting writes until it expires. With a public bucket those objects can be downloaded meanwhile.

**Suggested fix:** a per-user cap on bytes held in unattached files, a lower burst for users without Site Access, and a bucket lifecycle rule expiring `staging/` (STORAGE.md already suggests this). For S3, a presigned POST policy with `content-length-range` would bound the size.

### 11. `COOKIE_DOMAIN` advice sends the login token to every subdomain (Medium)

**Where:** `auth_cookie` in [auth/mod.rs:36-38](../backend/src/auth/mod.rs#L36-L38); the README's Docker setup, step 2.

The README suggests `COOKIE_DOMAIN=example.com` for `example.com` + `api.example.com`. The cookie then goes to every `*.example.com`: a wiki, a game panel, or a CDN or bucket on a custom subdomain. `gravnet_token` is a 24-hour bearer credential, so any of those hosts (or whoever runs or compromises them) can take over accounts. `HttpOnly` doesn't help against another server. The Origin check stops sibling subdomains from making writes, but not from reading the cookie.

**Suggested fix:** recommend a dedicated parent domain (`gnet.example.com` + `api.gnet.example.com`) in the README and `backend/.env.example`, and warn against putting other services under it.

### 12. Loading screen exposes private users' player records (Medium)

**Where:** `for_server` and `find_player` in [routes/loading_screen.rs:82-144](../backend/src/routes/loading_screen.rs#L82-L144); `LOOKUPS_PER_MINUTE` in [steam_profiles.rs:23](../backend/src/steam_profiles.rs#L23).

`GET /servers/{id}/loading-screen?steam_id=` is public, and returns any player's in-game name, usergroup, first join, last logon, playtime and gamemode stats (such as DarkRP money and job), plus their GravNet user id. It ignores the `private` profile setting. SteamIDs are shown on public profiles, so anyone can track a private user's activity across every server. Separately, the Steam lookup budget (60 a minute) is shared by everyone: requests for random SteamIDs keep it spent, so real players' loading screens lose their avatars.

**Suggested fix:** leave out `record` (or its stats and timestamps) for users with private profiles, and consider rate limiting cache misses per client IP rather than only globally.

### 13. No HSTS header (Low–Medium)

**Status:** Fixed in `4d597c0`. Caddy sends `Strict-Transport-Security: max-age=31536000` on both domains (without `includeSubDomains`, since other services may share the parent domain over HTTP), as do the backend when `BACKEND_URL` is HTTPS and the frontend in production, for setups without Caddy. Beyond the header, plain HTTP is now refused outside localhost: release builds of the backend won't start with plain-HTTP public URLs (or `COOKIE_SECURE=false`), production builds of the frontend won't build or serve with plain-HTTP backend URLs, and the addon sends nothing to a plain-HTTP `gravnet_api_url` on another machine unless `gravnet_allow_http` is set for development. See the README's "HTTPS" section.

**Where:** `security_headers` in [Caddyfile:19-27](../Caddyfile#L19-L27).

Caddy redirects HTTP to HTTPS but doesn't send `Strict-Transport-Security` unless told to, so a first visit or a typed `http://` link can be intercepted. The auth cookie is `Secure`, but the page and the login redirect aren't protected.

**Suggested fix:** add `?Strict-Transport-Security "max-age=31536000; includeSubDomains"` to `security_headers` (drop `includeSubDomains` if other subdomains may still serve HTTP).

### 14. No app-level rate limit on forum and support posting (Low–Medium)

**Status:** Fixed in `e00cb4e`. Per user: 3 forum threads per 10 minutes and 6 forum posts per minute (threads' first posts included; users with Thread Management exempt), and 6 support messages per minute for ticket owners (new tickets' first messages included; the Support Team exempt). Over the limit is a 429 with `Retry-After`. Votes aren't limited: they're stored on the post without timestamps, and each user has one per post.

**Where:** `create_thread`, `create_post` and `vote_post` in [routes/forum.rs:220](../backend/src/routes/forum.rs#L220), [:294](../backend/src/routes/forum.rs#L294), [:447](../backend/src/routes/forum.rs#L447); `create_message` in `routes/support.rs`.

Private messages, uploads and checkouts are rate limited per user, but forum threads, replies and votes, and support replies, aren't. Only Caddy's per-IP limit (600 requests a minute by default) stands in the way of a spammer.

**Suggested fix:** the same per-user window used for private messages (`PrivateMessage::recent_sent_by`).

### 15. Unbounded or expensive public reads (Low)

**Where:** `get_thread` in [routes/forum.rs:265-276](../backend/src/routes/forum.rs#L265-L276); `User::search` in [models/user.rs:197](../backend/src/models/user.rs#L197); `offset` in [routes/users.rs:390](../backend/src/routes/users.rs#L390).

- `GET /forum/threads/{id}` returns every post, and `GET /forum/categories/{id}` every thread, with no pagination.
- `GET /users/search` (no login needed) counts forum posts per user, sorts by it with `sort_by=posts`, and uses `ORDER BY random()` across every user when no name is given.
- `GET /users/{id}/posts` accepts any `offset`.

Cheap now, but at scale these are easy ways to load the database within the per-IP limit.

**Suggested fix:** paginate threads and categories, keep a post count on `users` (as reputation is kept), and cap `offset`.

## Permissions

### 16. Users of equal rank can ban each other (Low)

**Status:** Fixed in `4d597c0`. Banning, suspending and lifting need the issuer to strictly outrank the target: hold every permission the target's groups grant, and at least one more. Nobody outranks holders of every permission, so they can only be punished once removed from their groups. Moderators can no longer edit or delete posts by users holding permissions they don't. Peers can still remove each other from usergroups (that rule isn't strict), so a rogue admin can still demote other admins; recovering from that still needs the shell.

**Where:** `check_target` in [routes/punishments.rs:223-241](../backend/src/routes/punishments.rs#L223-L241); `manager` in [routes/forum.rs:485-494](../backend/src/routes/forum.rs#L485-L494).

A ban is refused only if the target holds a permission the issuer lacks, so two admins (or two moderators) can ban each other. A ban removes every permission, including the ability to lift it, so one rogue admin can lock out every other admin, and recovery needs the backend shell. Similarly, anyone with Thread Management can edit or delete anyone's posts, admins' included.

**Suggested fix:** if this isn't intended, require the issuer to hold strictly more than the target, or keep a protected group (such as `admin`) out of reach of web-issued bans.

### 17. Usergroup editors can restyle groups that outrank them (Low)

**Status:** Fixed in `4d597c0`. Editing a group needs every permission it grants, before and after the change, as deleting it does. The admin page shows such groups read-only.

**Where:** `Usergroup::update` in [models/usergroup.rs:471](../backend/src/models/usergroup.rs#L471).

`check_held` only looks at the permission bits that change. An editor with Usergroup Management can rename, recolor or change the icon and effect of a group holding permissions they don't have (only the default groups' names are protected).

**Suggested fix:** require the editor to hold all of a group's permissions to edit it at all, as deleting it already does.

## Privacy and Deployment

### 18. S3 private attachments readable at the public URL (Low)

**Where:** `read_target` in [storage.rs:377-391](../backend/src/storage.rs#L377-L391).

With `S3_PUBLIC_URL` set the bucket is public, so a support or private-message attachment can be read at `{S3_PUBLIC_URL}/files/{owner}/{id}` by anyone who has seen its presigned URL, which contains the key. Losing access later (staff removed from a group, for example) doesn't stop them. STORAGE.md documents this; the code comment on `read_target` reads as if it didn't apply.

**Suggested fix:** keep private attachments under a prefix the bucket policy doesn't make public (moving them there when attached), or document `S3_PUBLIC_URL` as unsuitable for communities that use private attachments.

### 19. Postgres password defaults to `gravnet` (Low)

**Status:** Fixed in `4d597c0`. A one-shot `db-password` Compose service generates a random 256-bit password into the `db-secrets` volume on first start (or writes `POSTGRES_PASSWORD`, if set). Postgres reads it through `POSTGRES_PASSWORD_FILE` and the backend through the new `DATABASE_PASSWORD_FILE`. Databases created with the old default need their password changed once; the README's "Updating" section has the command.

**Where:** [docker-compose.yml:20](../docker-compose.yml#L20), [:42](../docker-compose.yml#L42).

The database isn't published outside the Compose network, so the risk is low, but a forgotten password silently becomes `gravnet`.

**Suggested fix:** `${POSTGRES_PASSWORD:?set POSTGRES_PASSWORD in .env}`, as is done for the domains.

## Checked and Found Sound

These were reviewed and no problems were found:

- **Steam login:** the OpenID assertion is bound to a nonce cookie through `return_to` (preventing login CSRF), confirmed with Steam's `check_authentication`, and refused with duplicate parameters.
- **Tokens:** user JWTs are revoked by `token_version`; server tokens carry an audience, so neither kind passes for the other, and they rotate with a grace period.
- **CSRF:** cookie-authenticated writes from origins other than the frontend and backend are refused.
- **Stripe:** webhook signatures are checked in constant time with a timestamp tolerance; handlers are idempotent; the amount Stripe charged is checked against the purchase, holding delivery on a mismatch; the cancel link carries an HMAC.
- **Purchases:** item rows are locked while a purchase starts, stock is reserved and returned exactly once, and open checkouts per buyer are limited.
- **Usergroups:** editors can only grant or revoke permissions they hold, can't remove users who outrank them, and can't lock themselves out or empty the `admin` group. Grantable in-game usergroups need Elevated Control.
- **Markdown:** raw HTML is escaped and unsafe link schemes rejected on the backend; the frontend skips raw HTML as a second layer. Support tickets and private messages only show images from this backend.
- **Uploads:** local upload URLs are signed and expire; sizes are enforced; content types come from an allowlist; locally served files get `nosniff` and a sandboxing CSP.
- **Servers:** access keys are 256-bit, stored hashed and single-use; each addon route checks the token's server id against the path.
- **Addon delivery:** rewards are applied with `SetUserGroup`, ULib and `Give`, never through console command strings, so usergroup names and weapon classes can't inject commands.
- **General:** every SQL query is parameterized; outbound HTTP only goes to fixed Steam and Stripe endpoints (no SSRF); 5xx errors don't leak internals; `Config`, which holds secrets, is never logged.
