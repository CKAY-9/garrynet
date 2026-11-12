# Notifications

Part of [GarryNet Design](README.md).

Notifications tell a user that something happened that concerns them, and link to where they can act on it. The system is deliberately small: in-site only, one row per recipient, created by the backend when an event happens.

## Notification

One row per recipient (a notification for three users is three rows). This keeps read state, unread counts and deletion trivial, instead of tracking it in per-notification user id arrays.

```
id: number
user_id: number // recipient
type: NotificationType
title: string // short, plain text (128 characters max)
body?: string // plain text (512 characters max)
url?: string // site-relative path the notification links to, e.g. "/store/purchases"
created_at: utc timestamp
read_at?: utc timestamp // null = unread
```

`url` replaces the draft's free-form `actions` JSON. A notification links to one place; the page it opens is where the user acts.

Notifications are plain text, never Markdown or HTML, and `url` must start with `/`, so a notification can't link off-site.

## Notification Type

Used for the icon and for filtering. Types are fixed; add one when a new feature emits notifications.

```
NotificationType:
SUPPORT
STORE
FORUM
MESSAGE // private messages
ACCOUNT // punishments, usergroup changes
SYSTEM // anything else, e.g. admin messages
```

## Events

The backend creates notifications from the following events. The person who caused an event is never notified about it, so an admin replying to their own ticket gets nothing.

| Event | Recipient | Type | Links to |
|---|---|---|---|
| Support Team replies to a ticket (not internal notes) | ticket owner | SUPPORT | `/support/ticket/{id}` |
| Ticket resolved, closed or re-opened by the support team | ticket owner | SUPPORT | `/support/ticket/{id}` |
| Ticket closed automatically after 7 days awaiting the user | ticket owner | SUPPORT | `/support/ticket/{id}` |
| Owner replies to a ticket | ticket's `responders` | SUPPORT | `/support/ticket/{id}` |
| Purchase paid | buyer | STORE | `/store/purchases` |
| Fulfillment delivered in-game | buyer | STORE | `/store/purchases` |
| Fulfillment failed after 20 attempts | buyer | STORE | `/support/new` |
| Purchase refunded or fulfillment revoked | buyer | STORE | `/store/purchases` |
| Reply to a thread | thread author | FORUM | `/forum/threads/{id}` |
| Thread locked, or a post or thread deleted, by a moderator | post/thread author | FORUM | `/forum/threads/{id}` (none if deleted) |
| Private message received | recipient | MESSAGE | `/messages/{author_id}` |
| Banned | the user | ACCOUNT | `/support` |
| Suspended | the user | ACCOUNT | none |
| Ban or suspension lifted early | the user | ACCOUNT | none |
| Added to or removed from a usergroup | the user | ACCOUNT | `/users/{id}` |

Disputes (`charge.dispute.created`) are not a user notification. Staff handle them from the store admin pages.

## Rules

- **Collapse repeats.** If the recipient already has an unread notification with the same `type` and `url` from the same kind of event (e.g. five replies to one thread), update its `title`, `body` and `created_at` instead of adding a row. Otherwise one busy thread buries everything else.
- **Delivery failures don't block the action.** A notification is written after the triggering action commits. If writing it fails, log it and move on; replying to a ticket must never fail because of a notification.
- **Banned users still get notifications.** Notifications only require a login, not Site Access, because banned users can still use support and need to hear back about their ticket.
- **Retention.** The backend deletes read notifications after 30 days and any notification after 90 days, hourly with the other sweepers. A user keeps at most 200; the oldest read ones go first.
- **Deleted targets.** If the linked page no longer exists (a deleted ticket or thread), the notification stays until retention removes it; its link will 404. Notifications are cheap and this avoids cross-feature cleanup.

## Routes

Login is required. A user only ever sees and changes their own notifications; another user's id answers 404.

- GET `/notifications`: my notifications, newest first. Query: `unread` (`true` for unread only), `type`, `limit` (1 to 100, default 25), `cursor` (from the previous page's `next_cursor`).
- GET `/notifications/unread-count`: `{ count }`, for the navigation badge. Counts are capped at 99 in the UI.
- POST `/notifications/{id}/read`: mark one read. Idempotent.
- POST `/notifications/read-all`: mark all mine read. Takes an optional `type` to limit it.
- DELETE `/notifications/{id}`: delete one of mine.
- DELETE `/notifications`: delete all of mine that are read.

Creating notifications is internal; there is no public route. Admin announcements (a `SYSTEM` notification to every user) can be added later as an admin-only route, and should write one row per user in batches.

## Frontend

- **Navigation badge.** A bell with the unread count in the side navigation, above the profile preview (and in the mobile navigation). The count is fetched on page load and then every 60 seconds while the tab is visible. There are no websockets or push.
- **Dropdown.** Clicking the bell opens the 5 most recent notifications, each with its type icon, title and relative time, unread ones in bold, and a "View all" link. Clicking one marks it read and goes to its `url`.
- `/notifications`: the full list with an All / Unread toggle, a type filter, "Mark all as read" and "Delete read", and "Load more". Each row has mark read and delete buttons. Logged-out users are sent to log in.

- `/forum/threads/{id}`: forum notifications link here because they only know the thread. The page redirects to `/forum/{category_id}/{thread_id}` (404 if the thread is gone).

## Not included

Left out on purpose; add them only when needed:

- Email, Discord or in-game delivery. The addon could later show a "you have N unread" line, but that is a separate feature.
- Per-user preferences. A later addition is one mute toggle per `NotificationType`, applied when the notification is created. `ACCOUNT` and support replies should never be mutable.
- Saved or "acknowledged" notifications (`acknowledged_by` in the first draft). Read and delete cover it.
- Mentions. They need their own design first; each would add one event row above and no new schema.
- Real-time updates over websockets or server-sent events. Polling is enough at this scale.
