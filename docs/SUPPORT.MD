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
Currently attachments aren't considered, use links in content for now.

```
id: number
author_id: number
created_at: utc timestamp
last_updated: utc timestamp
content: string // markdown
previous_content?: string // stores null or the previous content if edited
ticket_id: number
is_internal: boolean // only responders are able to see this
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