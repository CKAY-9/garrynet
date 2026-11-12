# Store

Part of [GarryNet Design](README.md).

**GarryNet prohibits any usage of the store that involves gambling, loot boxes, or other
similar luck-based mechanics.** Item names and descriptions must not imply random or
chance-based rewards. Store managers are responsible for enforcing this on their items.

The store page allows players to buy in-game items and rewards for specific servers.
The store uses Stripe on the backend to handle payments and other associated services:
GarryNet opens a hosted [Checkout Session](https://docs.stripe.com/payments/checkout) for each
purchase and learns the outcome from Stripe's webhook.

## Configuration

Environment variables (set both or neither; without them the store can be browsed and managed,
but checkout answers 503 and item pages say purchases aren't set up):

- `STRIPE_SECRET_KEY`: the account's secret key (`sk_live_...`, `sk_test_...` or a restricted key).
- `STRIPE_WEBHOOK_SECRET`: the signing secret of a webhook endpoint pointed at
  `BACKEND_URL/stripe/webhook`. Signatures are always verified; unsigned requests are rejected.
  Subscribe it to `checkout.session.completed`, `checkout.session.async_payment_succeeded`,
  `checkout.session.async_payment_failed`, `checkout.session.expired`, `charge.refunded`,
  `charge.dispute.created` and `charge.dispute.closed`. Locally,
  `stripe listen --forward-to localhost:8080/stripe/webhook`.
- `STRIPE_TAX` (optional, default `false`): whether Checkout asks Stripe Tax to add tax. Whether
  Stripe Tax is enabled is a deployment decision.
- `STRIPE_TAX_BEHAVIOR` (optional, `exclusive` or `inclusive`, default `exclusive`): with
  `STRIPE_TAX`, whether item prices have tax added on top or already include it. Receipts record
  the item price either way.

If Stripe refuses the secret key (401/403), buyers only see "payments are unavailable"; the
details are logged.

No publishable key is needed: Checkout is hosted by Stripe, so no Stripe JavaScript runs on the
frontend. Refund policy and terms of sale should be linked from each item store page (the store
message supports plain text; put the links in the community's external links or an FAQ).

## Store Page

`/store` lists the servers that have an enabled item store. Logged-in users also get a
"My purchases" tab (`/store/purchases`). Users with Store Management see links to the admin pages.

## Eligibility

- Logged-out users can browse stores and items, but cannot buy.
- Buyers must be logged in with a linked Steam ID. The Steam ID is captured at purchase time.
- Users without `Site Access` (banned) cannot buy.
- Items that are unavailable or sold out, that the buyer has bought `max_per_user` times
  (paid or pending, not counting refunded purchases), that are free after discounts, or that grant
  a usergroup their store no longer allows can't be bought. The item page says why.

## Item Stores

Each registered Garry's Mod server can have at most one Item Store. These items can
be bought by players to give in-game rewards.

```
id: number
server_id: number // unique, one store per server
enabled: boolean // if false, hidden from /store and nothing can be bought
currency: string // ISO 4217, lowercase (e.g. "usd"), from the allowed list below; applies to every item in the store; fixed once it has items
override_name?: string // by default item stores use the server name
store_message?: string // shown at the top of the item store page
grantable_usergroups: string[] // allowlist of Garry's Mod usergroups items may grant; default empty
created_at: utc timestamp
last_updated?: utc timestamp
```

Stores may use USD, EUR, GBP, CAD, AUD, NZD, CHF, SEK, NOK, DKK, PLN, CZK, JPY, BRL or MXN
(`CURRENCIES` in `models/store.rs`, mirrored in the frontend's `lib/storeEnums.ts`). Prices are in
the currency's minor unit, which for JPY is the yen itself. A store set up before the list existed
keeps its currency but can't switch to another one off the list.

`grantable_usergroups` can only be edited by users with `Elevated Control`. This stops a store manager
from creating an item that grants a privileged rank such as `superadmin`. Removing a usergroup from
the list makes every item that grants it unavailable, and checkout re-checks each item's rewards
against the list, so an item saved before the change can't grant it. Public store responses return
`grantable_usergroups` empty; only the admin routes show it.

Deleting a server deletes its store, items, pending purchases and fulfillments. Receipts are kept
(see below). Stores themselves are never deleted on their own; disable one instead.

## Store Discounts

Item Stores are able to have discounts across the entire store. These discounts are additive
to pre-existing individual item discounts.

```
id: number
store_id: number
discount: number // 0-100 (e.g. 25 = 25% off)
message: string
created_at: utc timestamp
last_updated?: utc timestamp
starts_at?: utc timestamp
ends_at?: utc timestamp // if left empty, it's indefinite
active: boolean // whether now is within starts_at/ends_at; computed when read, not stored
```

**Discount calculation**

1. Sum the item's `discount` and the `discount` of all active store discounts.
2. Cap the total at 100.
3. `final_price = round(price * (100 - total) / 100)`, rounded to the nearest minor unit.

Items are returned with `total_discount` and `final_price` already computed. A total of 100% makes an
item free, which Stripe can't process: checkout answers 400 until the discount is adjusted.

## Store Item

Store Items are individual items that can be bought by players.

```
id: number
store_id: number
name: string
short_description: string // plain
full_description: string // markdown; may be empty
image_url?: string
sort_order: number // default = 0, ascending
created_at: utc timestamp
last_updated?: utc timestamp
available: boolean // if false, listed but no one can buy it
archived: boolean // soft delete; hidden everywhere but kept for receipts and fulfillment
price: number // integer, minor units of the store currency (e.g. cents)
discount: number // 0-100, default 0
starting_stock?: number // limited release items; null = unlimited
remaining_stock?: number // excludes reserved stock
max_per_user?: number // e.g. 1 for one-time items; counts paid and pending purchases, not refunded ones
all_time_purchases: number // default = 0; incremented when a purchase is paid, decremented when it's refunded
rewards: Reward[]
```

Prices are owned by GarryNet. The final price is calculated at checkout and passed to Stripe
with inline `price_data`, so no Stripe Price needs to be created or kept in sync.

Removing an item archives it. Items with receipts are never hard deleted. Changing
`starting_stock` moves `remaining_stock` by the same amount (never below zero), so units already
sold stay sold.

### Rewards

```
{ type: "usergroup", usergroup: string, duration_seconds?: number } // must be in the store's grantable_usergroups; permanent if no duration
{ type: "darkrp_money", amount: number }
{ type: "weapon", class: string } // permanent weapon
```

An item must have at least one reward (at most 20). Rewards are validated when the item is saved:
a `usergroup` must be grantable by the store, and `darkrp_money` is only valid when the server's
reported gamemode contains "DarkRP" (a server that hasn't sent a heartbeat yet is trusted).

## Purchases and Stock

A Purchase is one checkout attempt.

```
id: number
store_id: number
item_id: number
buyer_id: number
steam_id: string // the buyer's SteamID64 at purchase time
status: "PENDING" | "PAID" | "CANCELLED"
stripe_checkout_session_id?: string // unique
stripe_payment_intent_id?: string
quantity: number // always 1
list_price: number, discount_percentage: number, price: number, currency: string
rewards: Reward[] // snapshot of the item's rewards
stock_reserved: boolean // a unit of limited stock was taken for this purchase
created_at: utc timestamp
completed_at?: utc timestamp
```

1. The buyer starts checkout. GarryNet creates a `PENDING` purchase and a Stripe Checkout Session
   (35 minutes to pay; Stripe's minimum is 30) and sends the buyer to it. The price after every
   discount, and the currency, are read while the item is locked and recorded on the purchase;
   that's what Stripe charges.
2. If the item has limited stock, one unit is reserved while the item row is locked, so the last
   unit can't be sold twice and `max_per_user` can't be exceeded by parallel checkouts. If none is
   left, checkout fails. If Stripe refuses to open the session, or the session can't be recorded,
   the purchase is cancelled at once (and any session opened is expired).
3. `checkout.session.completed` (with `payment_status` paid) or `checkout.session.async_payment_succeeded`:
   the purchase becomes `PAID`, a Receipt is written, `all_time_purchases` is incremented, and a
   Fulfillment is created. The purchase is found by its session id or, if that was never recorded,
   by the `purchase_id` in the session's metadata. A paid GarryNet session whose purchase no longer
   exists (its server was deleted) is logged as an error for staff to refund.
4. `checkout.session.expired`, `checkout.session.async_payment_failed`, or the buyer backing out of
   Checkout (which lands on the frontend's `/store/checkout/cancel`): the purchase becomes
   `CANCELLED` and the reserved stock is released. Backing out also expires the Stripe session; if
   the session turns out to have been completed, the purchase is left for the webhook instead.
5. Every 10 minutes a sweeper asks Stripe about each purchase still pending after 90 minutes, in
   case its webhook never arrived. A paid session is recorded as paid (as the webhook would have);
   a completed session whose delayed payment (e.g. a bank debit) is still clearing is left pending,
   holding its stock, for up to 14 days; anything else is cancelled, expiring the session if it's
   somehow still open. If Stripe can't be reached the sweep stops until next time. A purchase that
   turns out to have been paid after being cancelled is reinstated when its webhook comes.

A buyer has at most one open checkout per item and 3 overall, so no one can hold an item's
limited stock by opening checkouts they never pay. Starting a checkout of an item closes the
buyer's previous open checkout of it first (unless that one was completed; then they're told to
check My purchases). Starting checkouts is also rate limited to 10 per buyer per 10 minutes.

The cancel link carries a token (an HMAC of the purchase id under `JWT_SECRET`), so another site
can't cancel a buyer's checkout by sending them to it.

If the pre-tax amount Stripe reports for a paid session differs from what the purchase recorded,
an error is logged for staff; the purchase is still recorded as paid.

Webhook handling is idempotent. Events are deduplicated using the unique Stripe ids on the Receipt
and the purchase's status, so Stripe may redeliver any event.

## Item Fulfillment

A player might not be in-game or in the correct server, so we need to track fulfillment.

```
id: number
receipt_id: number
buyer_id: number
steam_id: string
item_id?: number // null once the item was deleted with its server
server_id: number
rewards: Reward[] // snapshot at time of purchase; later item edits do not change it
status: "PENDING" | "FULFILLED" | "FAILED" | "REVOKED"
attempts: number // default = 0
last_error?: string
created_at: utc timestamp
fulfilled_at?: utc timestamp
revoked_at?: utc timestamp
revocation_applied_at?: utc timestamp // when the addon confirmed it took the rewards back
```

**Delivery**

- The server addon fetches its outstanding fulfillments using its rolling JWT, both when a player joins
  (`?steam_id=`) and periodically while they are connected.
- The addon applies the rewards, then acknowledges the fulfillment. Acknowledging `fulfilled` or
  `revoked` twice has no effect.
- If applying fails, the addon reports the error. `attempts` and `last_error` are updated and the
  fulfillment is offered again, up to 20 attempts. Each `failed` report counts as an attempt;
  reports after the 20th are ignored.
- Once a fulfillment has used up its attempts, a user with `Store Finance` can reset it from
  `/admin/store/receipts` ("Retry delivery") after fixing whatever made it fail.
- If Stripe reports charging a different pre-tax amount or currency than the purchase recorded
  (the webhook's `amount_subtotal`), the payment is still recorded, but the fulfillment is created
  held: failed, with no attempts left and a `last_error` explaining why. Nothing is delivered until a
  user with `Store Finance` has checked the payment in Stripe and retries it (or refunds it).
- A revoked fulfillment whose rewards were delivered is offered to the addon until it acknowledges
  taking them back.
- The addon's side (usergroup adapters, stacking timed usergroups, protecting staff groups, exactly-once
  delivery) is described in [ADDON.md](ADDON.md#delivery-in-the-addon).

## Receipt

Receipts are financial records: they outlive their item, store and server (the names are kept).

```
id: number
stripe_checkout_session_id: string // unique
stripe_payment_intent_id: string // unique
buyer_id: number
item_id?: number // null once the item was deleted with its server
item_name: string
store_id?: number
server_id?: number
store_name: string // the store's name at purchase time
quantity: number // default = 1
status: "PAID" | "REFUNDED" | "DISPUTED"
purchased_at: utc timestamp
list_price: number // minor units, before discounts
discount_percentage: number // total percentage applied
price: number // minor units, amount charged (before any tax Stripe added)
currency: string
status_changed_at?: utc timestamp
```

## Refunds and Disputes

- `charge.refunded` (for a full refund) sets the Receipt to `REFUNDED`. `charge.dispute.created` sets it to `DISPUTED`.
  Receipts only move forward (`PAID` to `DISPUTED` or `REFUNDED`, `DISPUTED` to `REFUNDED`), so
  redelivered or out-of-order events can't undo a refund.
- A won dispute (`charge.dispute.closed`) is only logged: the receipt stays `DISPUTED` and the
  rewards revoked until staff decide what to do.
- Either one sets the Fulfillment to `REVOKED`. If it was already fulfilled, the addon removes the
  granted usergroups and weapons on its next sync. Money rewards are not clawed back.
- Refunds are issued by users with `Store Finance` from `/admin/store/receipts`, which refunds the
  payment through Stripe and revokes the fulfillment immediately.
- Refunded stock is not automatically returned to `remaining_stock`. A refund does take the purchase
  off `all_time_purchases`, and it no longer counts towards the buyer's `max_per_user`.
- A payment already refunded from the Stripe dashboard is treated as refunded.

## Routes

Everything is JSON; errors are `{ "error": "..." }`. "Manage" means Admin Access and Store Management;
"Finance" means Admin Access and Store Finance.

**Public**

- GET `/store`: every enabled store with its `server`, `item_count` and `store_discount`
- GET `/store/{store_id}`: an enabled store with its `server`, active `discounts` and unarchived `items` (priced); 404 if disabled
- GET `/store/{store_id}/items/{item_id}`: `{ store, server, item, buyer }`; `buyer` (`{ purchased, can_buy, reason }`) is present when logged in

**Buyers** (login required)

- POST `/store/{store_id}/items/{item_id}/checkout`: starts a purchase, returns `{ purchase_id, url }` (201). Needs Site Access. 400 sold out / limit reached / free item / too many open checkouts, 409 a checkout of the item was started at the same moment, 423 unavailable, 429 rate limited, 422 Stripe refused, 503 Stripe not configured
- POST `/store/purchases/{purchase_id}/cancel`: `{ token }` (from the cancel link); cancels the buyer's own pending purchase, returns it. 404 for other people's purchases or a wrong token
- GET `/store/purchases`: the buyer's receipts, newest first, each with `buyer` and `fulfillment` (without `last_error`), and without Stripe's `stripe_checkout_session_id` and `stripe_payment_intent_id`, which only staff see

**Stripe**

- POST `/stripe/webhook`: `Stripe-Signature` required; 400 if invalid or older than 5 minutes

**Manage**

- GET `/admin/store`: every store (Manage or Finance)
- POST `/admin/store`: `{ server_id, enabled?, currency?, override_name?, store_message? }`; 409 if the server has a store; 400 for a currency off the allowed list
- GET `/admin/store/{id}`: the store with its `server`, every `discounts` and unarchived `items`
- PUT `/admin/store/{id}`: `{ enabled, currency, override_name, store_message }`; 400 if the currency changes while the store has items (archive them first) or to one off the allowed list
- PUT `/admin/store/{id}/grantable-usergroups`: `{ grantable_usergroups }`; also needs Elevated Control
- POST `/admin/store/{id}/items`, PUT `/admin/store/{id}/items/{item_id}`: `name`, `short_description`, `full_description`, `image_url`, `sort_order`, `available`, `price`, `discount`, `starting_stock`, `max_per_user`, `rewards`
- DELETE `/admin/store/{id}/items/{item_id}`: archives the item (204)
- POST `/admin/store/{id}/discounts`, PUT `/admin/store/{id}/discounts/{discount_id}`: `{ discount, message, starts_at, ends_at }`
- DELETE `/admin/store/{id}/discounts/{discount_id}`

**Finance**

- GET `/admin/store/receipts?status=&store=&buyer=`: receipts, newest first (at most 500); blank filters are ignored
- POST `/admin/store/receipts/{id}/refund`: full refund through Stripe; 400 unless the receipt is paid
- POST `/admin/store/fulfillments/{id}/retry`: resets a failed fulfillment's attempts so the addon tries again; returns the receipt. 400 unless it's failed and its receipt is paid

**Addon** (server JWT; see [ADDON.md](ADDON.md))

- GET `/gnet/server/{id}/fulfillments?steam_id=`: fulfillments to deliver, retry or revoke, oldest first
- POST `/gnet/server/{id}/fulfillments/{fulfillment_id}/ack`: `{ result: "fulfilled" | "failed" | "revoked", error? }`
- GET `/gnet/server/{id}/store?steam_id=`: the store's name and link (`null` if it has none or it's disabled) and the player's 10 most recent purchases on the server (none without `steam_id`), for the in-game `!gstore` menu and store chat messages

The admin dashboard's store statistics (`GET /admin/stats`) are `store.purchases_week` and
`store.income_week` (per currency, excluding refunds).

## Frontend

- `/store`, `/store/{id}`, `/store/{id}/items/{id}`: browsing; the item page's buy button opens Stripe
  Checkout, or explains why buying isn't possible. Stripe sends buyers back to `/store/purchases?paid=1`
  or `/store/checkout/cancel?purchase={id}&token={token}`.
- `/store/purchases`: receipts with delivery status.
- `/admin/store`: stores, create a store (Manage); link to receipts (Finance).
- `/admin/store/{id}`: settings, grantable usergroups (editable with Elevated Control), items, discounts.
- `/admin/store/{id}/items/new`, `/admin/store/{id}/items/{id}`: the item editor, including rewards, and archiving.
- `/admin/store/receipts`: receipts with filters and refunds (Finance).
