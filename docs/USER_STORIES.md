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

## Support

**Creating a new Ticket**
1. As a general user, I want to create a support ticket, so that I can get into contact with the support team
2. I do this by going to `/support`, and clicking New ticket
3. New ticket (`/support/new`) asks for title, subject, category, involved users, and the message.

**Reviewing my existing Tickets**
1. As a general user, I want to see all my previous and current tickets listed, so that I can view them in more detail
2. I do this by going to `/support` and viewing the list of tickets
3. I can sort and filter them

## Store

**Buying an Item**
1. As a logged in user, I want to buy an item from a store, so that I can receive in-game rewards.
2. I do this by going to `/store` and viewing the list of server stores (referred to as Item Stores)
3. Going into a store lists items and details like discount
4. Selecting an item lists all its description and rewards
5. Purchasing it goes through Stripe, which then hooks into GarryNet
6. GarryNet tracks and fulfills the purchase
7. If I am logged out or banned, I can browse but not buy.

**Cancelled or Failed Payment**
1. As a buyer, if I cancel or abandon payment, I want to return to the store with nothing charged.
2. Any stock reserved for me is released.

**Buying while Offline**
1. As a buyer, I want to buy an item while not in the server, so that I still get the reward.
2. The purchase is stored as a pending fulfillment.
3. The reward is delivered the next time I join that server.

**Viewing Purchase History**
1. As a logged in user, I want to see my past purchases and whether they were delivered.
2. I do this by going to `/store/purchases`, which lists receipts and fulfillment status.

**Creating an Item Store**
1. As a user with `Store Management`, I want to create a store for a registered server, so that I can create store items
2. I do this by going to `/admin/store`, navigable by admin dashboard
3. I select a server that doesn't have a store
4. This prompts for some optional information like message or override name

**Adding an Item**
1. As a user with `Store Management`, I want to create a item in a server store, so that players can purchase in-game rewards
2. I do this by selecting a item store in `/admin/store`
3. I input all necessary details and create it

**Editing, Disabling, and Archiving an Item**
1. As a user with `Store Management`, I want to edit an item, mark it unavailable, or archive it.
2. Archived items are hidden from players but kept for receipts and fulfillment history.

**Managing Store Discounts**
1. As a user with `Store Management`, I want to create a store-wide discount with an optional start and end time.
2. It is applied additively to individual item discounts and is capped at 100% total.

**Refunding a Purchase**
1. As a user with `Store Finance`, I want to view receipts and refund a purchase.
2. The refund revokes the associated fulfillment, removing granted usergroups and weapons.

## Forums

**Enable Templating**
1. As a user with post permissions, I want to toggle my thread to be a template, so that other
users can copy it
2. When enabled, other users can click use as template
3. I can do this at time of post creation or afterwards via editing

**Using Templates**
1. As a user with post permissions, I want to use a thread as a template, so that I can
follow it's structure
2. Clicking a post with templating enabled should redirect to a new thread form, with
title and content filled in. I can edit content and title, then post.

## Admin

**Add a User to a Usergroup**
1. As an admin with equivalent permission and `Usergroup Management`, I want to add a user to a usergroup
2. Going to `/admin/usergroups/{ID}` should have a members section.
3. I click "Add user" and am shown a search box with 10 matching results
4. I click the user and they're added to the group

**Viewing Users in Usergroup**
1. As an admin with equivalent permission and `Usergroup Management`, I want to view all users of a usergroup,
so that I can manage their membership
2. Going to `/admin/usergroups/{ID}` should list all users in a members section.
3. Each user has their name and icon, and an "X" (close) inline with them.
4. There should be a filter input to refine results (name or id)

**Removing User from Usergroup**
1. As an admin with equivalent permission and `Usergroup Management`, I want to remove a user from a usergroup
2. I view all members of a usergroup
3. I find the user and click the "X" next to their name
4. This removes them from the group