# Site Design

Part of [GarryNet Design](README.md). Frontend layout and visuals.

The whole site should have a consistent structure of [SIDE NAVIGATION] [CONTENT].

## Home Page

Should display community details (icon, description, name, splash, when applicable).
Should display key figures.
Should display any connected servers and a link to connect to them (`steam://`).

## Profile Page

Should display the user's avatar, name, description, status, when they joined and were last seen, reputation.

Display all usergroups they're a part of as "chips" (small, reusable components). They should implement the usergroup's effect attribute.

Should have a link to their steam profile. On your own profile an "Edit profile" button beside it opens `/settings`. Staff with Admin Access and User Management also get Ban/Unban and Suspend/Unsuspend buttons beside it, and see the user's punishments. A ban in effect (reason, when it was issued and when it ends, but not who issued it) is shown to everyone (see [USERS.md](USERS.md#banning-and-suspending)).

Below the stats, small previews of the user's threads and posts, with filters (threads or posts, a title/content search, and sorting by newest, oldest or reputation) and Previous/Next pages (see [USERS.md](USERS.md#profile-activity)).

A private profile shows other users only the avatar, name, status, usergroups, join date and last seen, with "This profile is private" in place of the bio, reputation and activity. Its owner and staff see everything, with a "Private profile" badge.

## Settings Page

`/settings`, for logged-in users who aren't banned (the 401 or 403 page otherwise). A User Settings section with the biography (plain text, with a character count) and a "Private profile" switch, saved together with Save; Discard restores the saved values (see [USERS.md](USERS.md#profile-settings)).

## Navigation

Community name and logo should always be displayed at the top of navigation.

### Side Navigation

Redirects in general should have inline icons and subtle hover/click animations.

The side navigation should stretch the whole height of the screen, and fit it. Should never take up >= 30% of the screen.

If it would take up too much screen space, switch to mobile-specific navigation.

More general redirects are higher up, e.g.,
1. Home
2. Forums
3. Store
4. Users
5. Support

Below the community name is a user search box. Submitting it opens `/users/search?name=...`.

User specific redirects are at the bottom, e.g.
- Notification bell with an unread badge (logged-in users only, above the Profile Preview). It opens a dropdown of recent notifications; see [Notifications](NOTIFICATIONS.md#frontend). On wide screens the dropdown is a flyout beside the sidebar; in the mobile overlay it opens inline.
- Profile Preview
- "Settings" link (gear icon), to `/settings`
- "Login with Steam" button (should include inline Steam icon), with a line under it: "By logging in you agree to our Terms and Privacy Policy."
- "Logout" button (should be stylized with a logout icon)
- Small links to the Terms, Privacy and Refunds pages, last (see [LEGAL.md](LEGAL.md))

### Mobile Navigation

Instead of being on the side, users now press a hamburger icon which reveals an overlayed navigation.

It should take up the whole screen, have overflow scroll if necessary, and slightly transparent/blur background.

## Users Page

Anyone should be able to grab a couple random users or filter search for them.

`/users` redirects to `/users/search`. The page has a filter form (name, user id, usergroup, last active, sort) above a grid of user cards. Each card shows the user's avatar with a status dot, their name, when they were last seen, up to three usergroup chips, and their reputation, post count, and id. Each card links to the user's profile.

## Themes

The site follows the operating system's light or dark preference (`prefers-color-scheme`). **Planned:** a manual toggle.

### Dark

The default theme: dark and cool, easy on the eyes.

### Light

A warmer, brighter theme.
