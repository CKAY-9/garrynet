# Site Design

Part of [GarryNet Design](DESIGN.md). Frontend layout and visuals.

The whole site should have a consistent structure of [SIDE NAVIGATION] [CONTENT].

## Profile Page

Should display the user's avatar, name, description, status, when they joined and were last seen, reputation.

Display all usergroups they're apart of as "chips" (small, reusable components). They should implement the usergroup's effect attribute.

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
4. Support

User specific redirects are at the bottom, e.g.
- Profile Preview
- "Login with Steam" button (should include inline Steam icon)
- "Logout" button (should be stylized with a logout icon)

### Mobile Navigation

Instead of being on the side, users now press a hamburger icon which reveals an overlayed navigation.

It should take up the whole screen, have overflow scroll if necessary, and slightly transparent/blur background.

## Themes

### Dark

The default theme: dark and cool, easy on the eyes.

### Light

A toggle switches to light mode, a warmer, brighter theme.
