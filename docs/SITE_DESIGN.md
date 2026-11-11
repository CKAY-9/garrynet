# Site Design

Part of [GarryNet Design](DESIGN.md). Frontend layout and visuals.

The whole site should have a consistent structure of [SIDE NAVIGATION] [CONTENT].

## Home Page

Should display community details (icon, description, name, splash, when applicable).
Should display key figures.
Should display any connected servers and a link to connect to them (`steam://`).

## Profile Page

Should display the user's avatar, name, description, status, when they joined and were last seen, reputation.

Display all usergroups they're apart of as "chips" (small, reusable components). They should implement the usergroup's effect attribute.

Should have a link to their steam profile.

## Navigation

Community name and logo should always be displayed at the top of navigation.

### Side Navigation

Redirects in general should have inline icons and subtle hover/click animations.

The side navigation should stretch the whole height of the screen, and fit it. Should never take up >= 30% of the screen.

If it were too take up too much screen space, switch to mobile specific navigation

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
- "Login with Steam" button (should include inline Steam icon)
- "Logout" button (should be stylized with a logout icon)

### Mobile Navigation

Instead of being on the side, users now press a hamburger icon which reveals an overlayed navigation.

It should take up the whole screen, have overflow scroll if necessary, and slightly transparent/blur background.

## Users Page

Anyone should be able to grab a couple random users or filter search for them.

`/users` redirects to `/users/search`. The page has a filter form (name, user id, usergroup, last active, sort) above a grid of user cards. Each card shows the user's avatar with a status dot, their name, when they were last seen, up to three usergroup chips, and their reputation, post count, and id. Each card links to the user's profile.

## Themes

### Dark

The default theme: dark and cool, easy on the eyes.

### Light

A toggle switches to light mode, a warmer, brighter theme.
