# User Stories

Part of [GarryNet Design](README.md).

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

**Viewing Purchase History in Server**
1. As a player in a linked GMod server, I want to run `!gstore`, so that I can view a summary list of my recent transactions
   - Player should be notified if the server doesn't have a store setup
2. Typing `!gstore`, opens the *GarryNet Store Menu* that primarily shows the users recent transactions. Details should be kept to necessities.

**Opening a Server's Store from GMod**
1. As a player in a linked GMod server, I want to run `!gstore`, so that I can get a link to the server's store if available
2. Opening the *GarryNet Store Menu*, I have a button that opens the link to the server's store

**Notifying Players of Store**
1. As an superadmin, I want to be able to toggle repeating messages alerting players to the store, so that I can generate revenue
2. Opening `!gnet` has a switch for Store chat messages, which are sent every 10 (configurable by superadmin) minutes.
3. The message should be editable by the superadmin
4. All changes should be able to be saved

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

## Profile Management

**Editing Description/Biography**
1. As a logged-in, non-banned user, I want to change my profile's bio, so that I can better reflect myself
2. Going to `/settings`, User Settings, displays possible configuration options, such as biography
3. Users can save it.

**Privating Profile**
1. As a logged-in, non-banned user, I want to toggle my privacy status, so that I can hide my posts, and reputation from other users (not admins)
2. Going to `/settings` has a privacy switch
3. This toggles user.private and hides my profile from other users

## Profile

**Viewing Profile Threads/Posts**
1. As a general user, I want to see a user's post on their profile, so that I can easily
follow them
   - If they are privated, I need to be an admin to see it
2. Going to a user's profile, `/users/${id}`, shows their details, and displays small previews of their posts and threads
3. I should be able to refine what I'm seeing by Post or Thread, title, content, reputation ascending/descending

## Private Messages

**Viewing Existing Messages**
1. As a logged-in user with `User Messages` permissions, I want to view my previous chats, so that I can engage with them
2. Going to `/messages` lists all previous chats with users

**Sending a Message to Someone New**
1. As a logged-in user with `User Messages` permissions, I want to create a new chat with a user, so that we can easily communicate
2. Going to `/messages` shows the user's chats, as well as a "New Message" button
3. User is able to search for a user by name or id, and send an initial message.

**Deleting a Chat**
1. Going into a chat has a header, where the other user's details are, that also has buttons
2. There is a trash can icon for deleting the chat, which creates a pop-up for user confirmation
3. Deleting it hides the existing messages from me only. The other user still sees the chat and isn't notified
4. Users are able to communicate again if they want: messages sent afterwards start a fresh chat

**Receiving a Message**
1. As a logged-in user with `User Messages` permissions, when someone sends me a message, I want to be alerted to it
2. A notification is created when someone messages the user, directing them to the new chat, `/messages/{author_id}`
3. Several messages from the same user collapse into one unread notification

**Seeing Unread Messages**
1. As a user with `User Messages` permissions, I want to see which chats have messages I haven't read
2. The side navigation's Messages link shows an unread badge, and `/messages` shows an unread count on each chat with unread messages
3. Opening the chat marks its messages as read and clears the count

**Sending a First Message**
1. As a user with `User Messages` permissions who has never sent a message, I want to be told that messages aren't really private
2. Opening the message page shows a popup explaining that staff can read messages, which I confirm. Until I do, the message box stays locked; closing the popup with "Not now" leaves a "Read and confirm" link to reopen it
3. Once I've sent a message, the popup no longer appears, and a visible, grayed-out disclaimer is shown on the message page instead
4. Deleting chats doesn't bring the popup back

**Sending a Message with Attachments**
1. As a user with `User Messages` permissions, I want to attach files to a message, so that I can share screenshots and clips
2. The message box supports markdown, up to 5,000 characters, and up to 3 attachments
3. Trying to attach a fourth file is refused
4. Only the sender, the recipient and staff with `Admin Access` and `User Management` can open the attachments

**Sending a Disappearing Message**
1. As a user with `User Messages` permissions, I want to mark a message as temporary, so that it doesn't stay in the chat
2. The message box has a "Disappearing message (24h)" toggle
3. After 24 hours the message is hidden from both me and the recipient
4. Staff can still see it, marked as expired

**Loading Older Messages**
1. As a user with `User Messages` permissions, I want to read a long conversation
2. Opening a chat, `/messages/{user_id}`, loads the 25 most recent messages
3. Scrolling upwards loads 25 more at a time until the start of the chat

**Messaging from a Profile**
1. As a user with `User Messages` permissions, I want to message someone while viewing their profile
2. Their profile, `/users/{id}`, has a "Message" button, which isn't shown on my own profile
3. It opens the chat with them, `/messages/{id}`, even if they've never messaged me
4. Their profile being private doesn't change this

**Messaging a Suspended or Banned User**
1. As a user with `User Messages` permissions, I try to message someone who is suspended or banned
2. They can't receive messages while the punishment is in effect, so sending is refused and my message isn't delivered
3. Messages already in the chat stay visible to both of us

**Viewing a User's Messages**
1. As an admin with `Admin Access` and `User Management`, I want to read a user's chats, so that I can look into reports and abuse
2. I do this by going to `/admin/messages`, navigable by admin dashboard
3. I search for a user by name or id, and their chats are listed
4. Opening a chat shows the messages read-only, including ones either user deleted and expired disappearing messages, which are marked as such
5. Each time I view a user's chats or a chat, it is logged, and the pages tell me so
6. A user's profile, `/users/{id}`, has a "View messages" link that opens their chats
