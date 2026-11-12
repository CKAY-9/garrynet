# Private Messages

Part of [GarryNet Design](README.md).

Private Messages let GarryNet users talk to each other on the platform. Sending and reading them requires the `User Messages` permission together with Site Access (see [PERMISSIONS.md](PERMISSIONS.md)). A [suspension](USERS.md#user-punishment) removes `User Messages`, and a ban removes everything, so punished users can't use messages until it ends.

There is no separate chat entity. A chat is just the pair of users who have exchanged messages, and it is addressed by the other user's id: `/messages/{user_id}`.

`user.private` only hides profile content (see [USERS.md](USERS.md#profile-settings)). It doesn't affect messaging: a user with a private profile can still send and receive messages.

## Private Message

Messages can't be edited in v1, so there is no `last_updated`. It would be added with an edit feature.

```
id: number
author_id: number
recipient_id: number // can't equal author_id
content: string // markdown, required, up to 5,000 characters
attachments: { owner_id, id, name, content_type, size }[] // max 3
temporary: boolean // default false; set by the sender, see Temporary Messages
deleted_by_author: boolean // default false; the author deleted the chat
deleted_by_recipient: boolean // default false; the recipient deleted the chat
read_at?: utc timestamp // null = the recipient hasn't opened it yet
created_at: utc timestamp
```

Markdown is sanitized like support messages (see [SUPPORT.md](SUPPORT.md#support-message)): raw HTML is escaped, and images can only be ones uploaded to GarryNet, i.e. URLs under `{BACKEND_URL}/file/`. Any other image is refused with 400. Staff can read messages, and an image from another host would tell whoever runs that host when they did. Links to other sites are fine.

Content is required and can't be blank, even with attachments. Up to 3 files can be attached (see [STORAGE.md](STORAGE.md)). The author uploads them first, then sends their file IDs as `attachments` with the message. Each must be a ready file in the author's own folder that isn't attached to anything else; otherwise 400.

## Messages Aren't Private

Messages aren't really private: staff with Admin Access and User Management can read them (see [Admins](#admins)). Users are told this:

- Until a user has sent their first message, a popup on the message page explains it, and they confirm before they can write.
- After that, a visible, grayed-out disclaimer stays on the message page.

Whether the popup shows isn't stored. The chat list returns `has_sent`, which is true if the user is the author of any message, including deleted and expired ones. Deleting a chat can't bring the popup back.

## Chats

A user's chat list holds one entry per other user they have messages with, newest first. Each shows the other user, a preview of the last message, when it was sent and how many messages are unread.

Selecting a chat loads the 25 most recent messages, oldest at the top. Scrolling upwards loads 25 more at a time. This is cursor pagination over `(created_at, id)`, so messages arriving while the user scrolls can't shift or repeat a page.

A message is **unread** until the recipient opens the chat, which sets `read_at` on everything the other user sent them. The side navigation shows an unread badge, and the chat list a dot on unread chats.

### Deleting a Chat

Users can delete a chat from its header, after confirming. This is per side and only affects that user's view:

- Deleting sets the user's own flag (`deleted_by_author` on messages they wrote, `deleted_by_recipient` on messages they received) on the messages that exist at that moment. The other user's view doesn't change and they aren't notified.
- Messages sent afterwards aren't flagged, so they start a fresh visible chat for both users. Either user can write again.
- Admins still see everything (see [Admins](#admins)).

## Temporary Messages

The sender can mark a message as **temporary** ("disappearing") when writing it. After 24 hours it is hidden from both users.

- This is enforced when messages are queried: a message is hidden if `temporary` is set and `created_at` is more than 24 hours ago. Nothing is deleted and there is no sweeper.
- Staff still see it, flagged as expired.
- Its attachments are kept.
- An expired message still counts for `has_sent`.
- A chat whose only visible messages have expired no longer appears in either user's chat list.

## Sending

Requires the sender to have Site Access and `User Messages`. Rules:

- The recipient must exist and can't be the sender: 404 for an unknown user, 400 for yourself.
- The recipient must also have Site Access and `User Messages` (effective permissions, so suspended and banned users count). Otherwise a generic 403 that doesn't say why.
- Content is 1 to 5,000 characters and up to 3 attachments (400 otherwise).
- At most 10 messages per 30 seconds per sender; 429 with `Retry-After` beyond that.

The recipient gets a notification (see below). It is written after the message commits, and a failure to write it doesn't fail the send (see [NOTIFICATIONS.md](NOTIFICATIONS.md#rules)).

### Notifications

Sending creates a `MESSAGE` notification for the recipient (event `message.received`) linking to `/messages/{author_id}`. Several messages from the same user collapse into one unread notification, as described in [NOTIFICATIONS.md](NOTIFICATIONS.md#rules). The title names the author and the body is the start of the message as plain text. The author is never notified of their own messages.

### Attachment Privacy

Attachments are as private as the chat: once attached, a file is only served to the message's author, its recipient and staff with Admin Access and User Management. Everyone else, logged out or not, gets 404, even with the link. This applies even if the author or recipient deleted the chat, and to expired messages. See [STORAGE.md](STORAGE.md#get-fileowner_idfile_id).

## Routes

Login is required for all of them. The first six also require Site Access and `User Messages`; the admin ones require Admin Access and User Management. `{user_id}` is the other user in the chat.

| Route | Purpose |
|---|---|
| GET `/messages/chats` | My chat list, newest first, with `has_sent`. Query: `limit` (1 to 100, default 25), `cursor` |
| GET `/messages/with/{user_id}` | My messages with that user, newest first. Query: `limit` (1 to 100, default 25), `cursor` (from the previous page's `next_cursor`) |
| POST `/messages/with/{user_id}` | Send a message: `{ content, attachments?, temporary? }`. Returns the message (201) |
| POST `/messages/with/{user_id}/read` | Mark what they sent me as read. Idempotent |
| DELETE `/messages/with/{user_id}` | Delete the chat for me only (204) |
| GET `/messages/unread-count` | `{ count }`, for the navigation badge |
| GET `/messages/admin/{user_id}/chats` | Admin: that user's chats, including deleted and expired ones |
| GET `/messages/admin/{user_id}/with/{other_id}` | Admin: one chat between two users, including deleted and expired messages. Same pagination |

Reads return `{ items, next_cursor }` like [notifications](NOTIFICATIONS.md#routes); `next_cursor` is null on the last page. A bad `cursor` or out-of-range `limit` is 400.

## Admins

Staff with Admin Access and User Management can pick a user and read their chats at `/admin/messages` (see [ADMIN.md](ADMIN.md#adminmessages)). This is read-only: staff can't send, edit or delete messages on someone's behalf.

- The admin views show everything: messages either side deleted, and temporary ones past 24 hours. Each message carries `deleted` and `expired` flags so the page can mark them.
- Every admin view of a user's chats or a chat is logged with the staff member and the users involved. This is the only audit trail.
- Staff can read private messages, and the privacy policy template says so (see [LEGAL.md](LEGAL.md)).

## Frontend

- `/messages`: my chats with unread dots and a "New Message" button, which opens a user search by name or id (reusing the support ticket user search) and then the chat. Requires `User Messages`.
- `/messages/{user_id}`: the chat. A header with the other user and a delete (trash can) button, the messages, and a composer with markdown text, up to 3 attachments and a "disappearing (24h)" toggle. The first send shows the popup, and the grayed-out disclaimer stays afterwards.
- Newer messages are fetched about every 15 seconds while the tab is visible. There are no websockets or push (see [Not included](#not-included)).
- Side navigation: a "Messages" link with an unread badge, shown to users with `User Messages` (see [SITE_DESIGN.md](SITE_DESIGN.md#side-navigation)).
- User profiles: a "Message" button for viewers with `User Messages` who aren't the profile's owner (see [USERS.md](USERS.md#messaging)).
- `/admin/messages`: the staff view described above.

## Not included

Left out on purpose; add them only when needed:

- Editing messages (and with it `last_updated` and an edit history).
- Blocking users, and reporting a message to staff. Staff punishments already revoke messaging, and users can open a support ticket in the meantime.
- Websockets, server-sent events or push. The chat polls for new messages.
- Group chats. A chat is always exactly two users.
- Email, Discord or in-game delivery of messages.
