# User Stories

Part of [GarryNet Design](DESIGN.md).

## General

**Splash Page**

1. As a general user, I should see the side navigation and the community's splash page.
   - The splash page shows the community name, description, background, and general information.
2. I should be able to navigate to `/forum`, `/store`, `/players`, and `/loading-screen`, and see a "Login with Steam" button.

**Viewing Profile**

1. As a logged-in user, I should see my profile in place of the "Login with Steam" button, so that I can navigate to it.
2. Navigating to my profile should redirect to `/users/{USER_ID}`, where `USER_ID` is my own GarryNet user id.

**Admin View**

1. As a logged-in user with the `Admin Access` permission, I should see the `/admin` navigation item.
2. Clicking it should take me to `/admin` without being redirected away.

## User Login

**Regular Flow**

1. As a logged-out user, I want to click the "Login with Steam" button, so that I am redirected to Steam's login flow.
2. Steam redirects to the backend (`/auth/steam/callback`), which signs the user in with a JWT.
3. The user is redirected to the frontend. Next.js can read the JWT server-side and use it to fetch the user's data.

## User Searching

**On Users Search Page `/users/search`**

1. As a general user, I want to search up a user, so that I can view their profile
2. I should be able to filter by name, usergroup, last activity, id, and sort by most, least reputation, or number of posts.
3. Doing so will show the filtered users that I can engage with.

## Loading Screen

**Loading into Garry's Mod server**

1. As a player, I want the loading screen to fetch my player information for the server, so that I can see gamemode attributes, my last logon, and my Steam details.

**Viewing on `/loading-screen`**

1. As a general user, I want the loading screen to show a fake user and fake gamemode/server details, so that I can preview it.

## Garry's Mod Servers

**Registering a Garry's Mod Server**
1. As a user with `Server Management`, I want to register a new server, so that I can link it later.
2. I first provide a name and description.
3. I am given a generated access key to use on a server running the GarryNet addon.

**Linking a Garry's Mod Server**
1. As a Garry's Mod superadmin, I want to paste the generated `access_key`, so that I can link this server to GarryNet
2. Run `!gnet`
3. If the server is already linked to an instance of GarryNet, give options, otherwise have a text box for `access_key`
4. The key is sent to the server and verified, then the server is linked and given a rolling JWT.