# Support

Part of [GarryNet Design](DESIGN.md).

GarryNet offers a support system to users to get into contact with community administrators or review frequently asked questions. Tickets should support markdown like forum posts.

## Support Ticket

Users are able to create support tickets as needed. No user should ever have more than 5 active support tickets at once (i.e. not closed/resolved).

Permission for admins: `Support Team`

```
id: number
title: string
subject: string // short description for glancing (256 characters max)
user_id: number
associated_user_ids: number[] // if the ticket involved other users, store the ids. this is provided by the user at the time of ticket creation via a dropdown
initial_message_id: number
created_at: utc timestamp
last_updated: utc timestamp
category: TicketCategory
status: SupportStatus
resolved_at?: utc timestamp
closed_at?: utc timestamp
closed_by?: number // user id, also covers if it was resolved and not closed
responders: number[] // support/admin team members who have responded to the ticket, should automatically update on message/status change by any admin
```

## Ticket Category

Enum for tickets.

Tickets should be filtered into specific categories.

```
TicketCategory:
OTHER
USER_REPORT // involved other users/players
SUGGESTION 
BUG_REPORT
USER_ISSUE // didn't receive store item reward, etc.
```

## Support Status

Enum for tracking ticket status

- OPEN: the status when initially created. No message other than the initial message.
- AWAITING_USER: when an administrator responds and is waiting for the user to send a reply
- AWAITING_ADMIN: when a user responds and is waiting for the admin team to reply
- RESOLVED: successfully resolved the issue
- CLOSED: unknown resolution, but no continuation

Resolved and Closed tickets can only be re-opened/responded to by the support team.

Tickets should close automatically after 7 days of no activity from the user (i.e. status = AWAITING_USER). Automatic closing is not a resolution, status = CLOSED.

```
SupportStatus:
OPEN
AWAITING_USER
AWAITING_ADMIN
RESOLVED
CLOSED
```

## Support Message

Support messages can be edited once and have the previous value in `previous_content`.
A message can have up to 10 files attached (see [STORAGE.md](STORAGE.md)). The author uploads them first, then sends their file IDs as `attachments` with the message (or with the ticket, for its initial message). Each must be a ready file in the author's own folder that isn't attached to anything else; otherwise 400. Editing a message doesn't change its attachments.

File URLs are public but unguessable, so an attachment can be opened by anyone who is given its link, even though the ticket itself is private.

```
id: number
author_id: number
created_at: utc timestamp
last_updated: utc timestamp
content: string // markdown
previous_content?: string // stores null or the previous content if edited
ticket_id: number
is_internal: boolean // only responders are able to see this
attachments: { owner_id, id, name, content_type, size }[]
```

## /support Page

For all users, list my:
- On-going tickets: tickets that haven't been closed/resolved
- Resolved/Closed tickets: tickets that have been addressed
- FAQ Grid: should grab 3 FAQs and display them, with a way to access more

Usergroups with `Support Team`:
- Include a header/navigation for switching between my tickets and support tickets (admin view)

All users should be able to filter/sort.

## /support/admin

Admins should be able to see all support tickets and filter them by:
- status
- category
- latest activity

## /support/ticket/{ID}

Fetch all messages associated with the support ticket, its status, and details. 
List out people who have responded or engaged with the ticket.

Users are able to create new messages for the ticket, updating its status.

Admins have a menu for closing or resolving the ticket.

## FAQs

FAQs are used to address general problems, complaints, and issues that may arise.

```
id: number
title: string
subject: string // short description (256 characters max)
category: FAQCategory
description: string // markdown
created_at: utc timestamp
last_updated: utc timestamp
created_by: number
editors: number[]
```

## FAQ Category

Enum for organizing FAQs

```
FAQCategory:
GENERAL
SUPPORT
STORE
FORUM
USER
COMMUNITY
```

## /support/faq

List FAQs by category, include:
- title
- subject
- category
- last_updated 
- and all editors/creator icons

## /support/faq/{ID}

Show everything for the FAQ.
## Routes

Login is required for tickets but not Site Access: banned users can still use support. "Support Team" means the `Support Team` permission.

Tickets:
- GET `/support/tickets`: my tickets. Query: `status`, `category`, `state` (`active` | `done`), `activity` (`day` | `week` | `month`, updated within), `sort` (`latest` default | `oldest` | `created`). Each ticket includes its `author` and visible `message_count`.
- POST `/support/tickets`: open a ticket with its initial message. Takes `title`, `subject`, `category`, `associated_user_ids` (max 10, must exist), `content` (Markdown) and optional `attachments` (up to 10 file IDs). 400 once the user has 5 active tickets.
- GET `/support/admin/tickets`: every ticket, same query as above. Requires Support Team.
- GET `/support/tickets/{id}`: the ticket, its `author`, `associated_users`, `responders`, `closed_by_user` and `messages` (internal ones only for Support Team). Owner or Support Team; 404 otherwise.
- POST `/support/tickets/{id}/messages`: add a message (`content`, `is_internal`, optional `attachments`, up to 10 file IDs). Owners move the ticket to `AWAITING_ADMIN` (423 once resolved/closed); Support Team replies move it to `AWAITING_USER` and re-open it, internal notes change nothing else. Responders update automatically.
- PATCH `/support/tickets/{id}/status`: set `status`. `RESOLVED`/`CLOSED` stamp `resolved_at`/`closed_at` and `closed_by`; any other status re-opens the ticket. Requires Support Team.
- PATCH `/support/messages/{id}`: edit a message's `content`, once (author only; owners not once the ticket is resolved/closed).

Tickets in `AWAITING_USER` with no activity for 7 days are closed by the backend hourly (`closed_by` stays null).

FAQs:
- GET `/support/faqs`: all FAQs, most recently updated first; `category` and `limit` (1 to 100) narrow them. Includes `creator` and `editor_users`. Public.
- GET `/support/faqs/{id}`: one FAQ. Public.
- POST `/support/faqs`, PUT `/support/faqs/{id}`, DELETE `/support/faqs/{id}`: manage FAQs (`title`, `subject`, `category`, `description`). Editing records the editor. Requires Support Team.

## Frontend

- `/support`: my on-going and resolved/closed tickets with filters, three FAQs, and a "New ticket" link. Logged-out users see the FAQs and a login prompt. Tabs switch between my tickets, all tickets (Support Team only) and FAQs.
- `/support/new`: the ticket form, with a user search for associated users.
- `/support/admin`: every ticket with the same filters (Support Team).
- `/support/ticket/{id}`: messages, reply box, details and participants. Support Team sees internal notes, can add them, and has resolve/close/re-open buttons.
- `/support/faq`, `/support/faq/{id}`, `/support/faq/new`: FAQs by category; Support Team creates, edits and deletes them.
