export type UserStatus = "ONLINE" | "IDLE" | "OFFLINE";

/** A GarryNet user, as returned by the backend. Timestamps are ISO 8601 (UTC). */
export type User = {
  id: number;
  username: string;
  email: string | null;
  avatar_url: string;
  steam_id: string;
  bio: string;
  reputation: number;
  status: UserStatus;
  joined: string;
  last_seen: string;
};

/** The logged-in user, from `GET /users/me`: their own account plus effective permissions. */
export type CurrentUser = User & {
  /** Bit flags; see `@/lib/permissions`. */
  permissions: number;
};

/** Dashboard statistics, from `GET /admin/stats`. */
export type AdminStats = {
  users: {
    registered: number;
    /** Users currently ONLINE. */
    active: number;
  };
  forum: {
    /** Posts made in the past 7 days. */
    posts_week: number;
  };
  store: {
    /** Paid purchases in the past 7 days. */
    purchases_week: number;
    /** Revenue in the past 7 days per currency (minor units), excluding refunds. */
    income_week: { currency: string; amount: number }[];
  };
};

export type UsergroupEffect = "halo" | "rainbow" | "flash" | "none";

/** The parts of a usergroup needed to display it, e.g. as a chip on a profile. */
export type UsergroupBadge = {
  id: number;
  name: string;
  effect: UsergroupEffect;
  /** Hex color, e.g. `#e5484d`. */
  color: string;
  icon_url: string | null;
};

/** A usergroup in full, from `GET /admin/usergroups` (needs Usergroup Management). */
export type Usergroup = UsergroupBadge & {
  /** Ids of the users in this group. */
  members: number[];
  /** Permission bit flag; see `Permission`. */
  permissions: number;
  created_at: string;
  last_updated: string | null;
};

/** A member of a usergroup, from `GET /admin/usergroups/{id}/members`. */
export type UsergroupMember = {
  id: number;
  username: string;
  avatar_url: string;
  /** When they joined the group. */
  added_at: string;
};

/** A usergroup's editable fields, sent to `POST`/`PUT /admin/usergroups`. */
export type UsergroupDetails = Pick<Usergroup, "name" | "effect" | "color" | "permissions" | "icon_url">;

/**
 * A user's public profile, from `GET /users/{id}`. Excludes the email; `status` accounts
 * for inactivity (idle after 5 minutes, offline after 15).
 */
export type UserProfile = Omit<User, "email"> & {
  usergroups: UsergroupBadge[];
};

/** A user as listed by `GET /users/search`: public profile basics plus their post count. */
export type UserSearchResult = Pick<
  UserProfile,
  "id" | "username" | "avatar_url" | "reputation" | "status" | "joined" | "last_seen" | "usergroups"
> & {
  post_count: number;
};

/** `last_activity` filter: seen within this window (`online` = currently online). */
export type LastActivity = "online" | "hour" | "day" | "week" | "month";

/** `sort_by` for user search: most / least reputation, or most posts. */
export type UserSort = "reputation+" | "reputation-" | "posts";

/** Filters for `GET /users/search`. Blank values are ignored. */
export type UserSearchFilters = {
  name?: string;
  id?: string;
  /** Usergroup id. */
  usergroup?: string;
  last_activity?: LastActivity | "";
  sort_by?: UserSort | "";
};

export type LoadingScreenStyle = "aero" | "sleek";

/** The loading screen settings, from `GET /loading-screen`. Shared by every server. */
export type LoadingScreenConfig = {
  id: number;
  style: LoadingScreenStyle;
};

/** A Garry's Mod server, as returned by the backend (never includes its access key). */
export type Server = {
  id: number;
  name: string;
  description: string;
  /** Empty until the server's first heartbeat. */
  gamemode: string;
  splash_url: string | null;
  icon_url: string | null;
  /** Hex color, e.g. `#3b82f6`. */
  color: string;
  connection_address: string;
  /** Empty until the server's first heartbeat. */
  map: string;
  last_heartbeat: string | null;
  linked: boolean;
  created_at: string;
};

/** A player's record on one server, kept up to date by the addon. */
export type UserPlayer = {
  id: number;
  server_id: number;
  steam_id: string;
  user_id: number | null;
  ingame_name: string;
  /** Equals `first_joined` until the player returns. */
  last_logon: string;
  first_joined: string;
  ingame_usergroup: string;
  /** Gamemode-specific data, e.g. `{ darkrp_money: 5000 }`. */
  stats: Record<string, unknown>;
  /** Minutes. */
  time_played: number;
};

/** The player who is loading in. */
export type LoadingScreenPlayer = {
  steam_id: string;
  /** GarryNet username, else the in-game name, else the Steam name. */
  name: string;
  avatar_url: string | null;
  user_id: number | null;
  /** Their record on this server; `null` on their first visit. */
  record: UserPlayer | null;
};

/** Everything one server's loading screen shows, from `GET /servers/{id}/loading-screen`. */
export type LoadingScreenData = {
  style: LoadingScreenStyle;
  server: Server;
  /** `null` when the page was opened without a valid SteamID64. */
  player: LoadingScreenPlayer | null;
};

/** A notable member of the community, shown on the homepage. */
export type KeyFigure = {
  /** Their GarryNet account, if they have one. Absent otherwise. */
  user_id?: number;
  name: string;
  /** Image URL; empty for none. */
  profile_picture: string;
  role: string;
  description: string;
};

/** The community this deployment serves, from `GET /community`. */
export type Community = {
  id: number;
  name: string;
  description: string;
  logo_url: string | null;
  /** Image URL shown behind the homepage header. */
  splash_background: string | null;
  external_links: string[];
  key_figures: KeyFigure[];
  created_at: string;
  last_updated: string | null;
};

/** The editable parts of the community, sent to `PUT /community`. */
export type CommunityDetails = Pick<
  Community,
  "name" | "description" | "logo_url" | "splash_background" | "external_links" | "key_figures"
>;

/** The parts of a server admins edit, sent to `POST /servers` and `PUT /servers/{id}`. */
export type ServerDetails = Pick<
  Server,
  "name" | "description" | "splash_url" | "icon_url" | "color" | "connection_address"
>;

/** A server with a newly generated access key; the only time the backend returns the key. */
export type ServerWithAccessKey = {
  server: Server;
  access_key: string;
};

/** A top-level grouping on the forum homepage. */
export type ForumSection = {
  id: number;
  name: string;
  /** Hex color. */
  accent: string;
  icon_url: string | null;
  /** Max 256 characters. */
  description: string;
  /** Listed highest to lowest; -1 is always last. */
  position: number;
  created_at: string;
  last_updated: string | null;
};

export type ForumCategory = {
  id: number;
  name: string;
  section_id: number;
  /** Max 256 characters. */
  description: string;
  /** Prevents new threads from being created. */
  locked: boolean;
  /** Same ordering rules as `ForumSection`. */
  position: number;
  created_at: string;
  last_updated: string | null;
};

export type ForumThread = {
  id: number;
  author_id: number;
  category_id: number;
  title: string;
  /** Id of the thread's first post. */
  thread_post_id: number;
  created_at: string;
  last_activity: string;
  /** Prevents any changes to the thread. */
  locked: boolean;
  /** Listed before non-sticky threads. */
  sticky: boolean;
  /** Users can copy this thread as a template for new threads. */
  allow_template: boolean;
};

export type ForumPost = {
  id: number;
  author_id: number;
  thread_id: number;
  content: string;
  created_at: string;
  edited: string | null;
  /** Ids of users who liked the post. */
  likes: number[];
  dislikes: number[];
  /** Files attached when the post was written. */
  attachments: Attachment[];
};

/** The author fields the forum displays next to threads and posts. */
export type ForumAuthor = Pick<User, "id" | "username" | "avatar_url">;

/** A category with its activity, as listed on the forum homepage. */
export type ForumCategorySummary = ForumCategory & {
  thread_count: number;
  post_count: number;
  /** The most recently active thread, if any. */
  latest: { id: number; title: string; last_activity: string; author: ForumAuthor } | null;
};

/** The forum homepage, from `GET /forum`. Sections and categories are in display order. */
export type ForumHome = {
  sections: (ForumSection & { categories: ForumCategorySummary[] })[];
  stats: { threads: number; posts: number; members: number; online: number };
};

/** A thread as listed in its category. */
export type ForumThreadSummary = ForumThread & { author: ForumAuthor; replies: number };

/** A category page, from `GET /forum/categories/{id}`. Stickies first, then most recently active. */
export type ForumCategoryPage = {
  category: ForumCategory;
  section: ForumSection;
  threads: ForumThreadSummary[];
};

export type ForumPostWithAuthor = ForumPost & { author: ForumAuthor };

/** A thread page, from `GET /forum/threads/{id}`. The first post, then replies oldest first. */
export type ForumThreadPage = {
  category: ForumCategory;
  section: ForumSection;
  thread: ForumThread;
  posts: ForumPostWithAuthor[];
};

/** Sent to `POST /forum/sections`. */
export type ForumSectionDetails = Pick<ForumSection, "name" | "accent" | "icon_url" | "description" | "position">;

/** Sent to `POST /forum/categories`. */
export type ForumCategoryDetails = Pick<ForumCategory, "name" | "section_id" | "description" | "locked" | "position">;

// Support (docs/SUPPORT.MD)

export type TicketCategory = "OTHER" | "USER_REPORT" | "SUGGESTION" | "BUG_REPORT" | "USER_ISSUE";

export type SupportStatus = "OPEN" | "AWAITING_USER" | "AWAITING_ADMIN" | "RESOLVED" | "CLOSED";

export type FaqCategory = "GENERAL" | "SUPPORT" | "STORE" | "FORUM" | "USER" | "COMMUNITY";

/** The user fields the support pages display next to tickets, messages and FAQs. */
export type UserRef = Pick<User, "id" | "username" | "avatar_url">;

export type SupportTicket = {
  id: number;
  title: string;
  /** Short description, max 256 characters. */
  subject: string;
  user_id: number;
  /** Other users the ticket involves. */
  associated_user_ids: number[];
  initial_message_id: number;
  created_at: string;
  last_updated: string;
  category: TicketCategory;
  status: SupportStatus;
  resolved_at: string | null;
  closed_at: string | null;
  /** Who resolved or closed it; `null` when closed automatically. */
  closed_by: number | null;
  /** Support team members who have responded to or changed the ticket. */
  responders: number[];
};

/** A ticket as listed by `GET /support/tickets` and `GET /support/admin/tickets`. */
export type TicketSummary = SupportTicket & {
  author: UserRef;
  /** Messages the requester can see. */
  message_count: number;
};

export type SupportMessage = {
  id: number;
  author_id: number;
  created_at: string;
  last_updated: string;
  content: string;
  /** What the message said before its one allowed edit. */
  previous_content: string | null;
  ticket_id: number;
  /** Only the support team sees internal messages. */
  is_internal: boolean;
  /** Files attached when the message was written. */
  attachments: Attachment[];
};

export type SupportMessageWithAuthor = SupportMessage & { author: UserRef };

/** A ticket page, from `GET /support/tickets/{id}`. Messages oldest first. */
export type TicketPage = {
  ticket: SupportTicket;
  author: UserRef;
  associated_users: UserRef[];
  responders: UserRef[];
  closed_by_user: UserRef | null;
  messages: SupportMessageWithAuthor[];
  /** Whether the requester is on the Support Team. */
  support: boolean;
};

/** Sent to `POST /support/tickets`. */
export type TicketDetails = Pick<SupportTicket, "title" | "subject" | "category" | "associated_user_ids"> & {
  /** The initial message, Markdown. */
  content: string;
  /** IDs of uploaded files to attach to the initial message. */
  attachments: string[];
};

/** Filters for ticket lists. Blank values are ignored. */
export type TicketFilters = {
  status?: SupportStatus | "";
  category?: TicketCategory | "";
  /** Updated within this window. */
  activity?: TicketActivity | "";
  sort?: TicketSort | "";
};

export type TicketActivity = "day" | "week" | "month";

export type TicketSort = "latest" | "oldest" | "created";

export type Faq = {
  id: number;
  title: string;
  /** Max 256 characters. */
  subject: string;
  category: FaqCategory;
  /** Markdown. */
  description: string;
  created_at: string;
  last_updated: string;
  created_by: number;
  editors: number[];
};

/** An FAQ with its creator and editors, from `GET /support/faqs`. */
export type FaqWithPeople = Faq & {
  creator: UserRef;
  editor_users: UserRef[];
};

/** Sent to `POST /support/faqs` and `PUT /support/faqs/{id}`. */
export type FaqDetails = Pick<Faq, "title" | "subject" | "category" | "description">;

// Store (docs/STORE.md)

/** What an item gives the buyer in-game. */
export type Reward =
  | {
      type: "usergroup";
      /** Must be one of the store's `grantable_usergroups`. */
      usergroup: string;
      /** Permanent when absent. */
      duration_seconds?: number | null;
    }
  | { type: "darkrp_money"; amount: number }
  | { type: "weapon"; class: string };

export type RewardType = Reward["type"];

/** A server's store. Named after the server unless `override_name` is set. */
export type ItemStore = {
  id: number;
  server_id: number;
  /** Hidden from `/store` and nothing can be bought while false. */
  enabled: boolean;
  /** ISO 4217, lowercase, e.g. `usd`. Applies to every item. */
  currency: string;
  override_name: string | null;
  /** Shown at the top of the store page. */
  store_message: string | null;
  /** Garry's Mod usergroups items may grant. Only Elevated Control changes it. */
  grantable_usergroups: string[];
  created_at: string;
  last_updated: string | null;
};

/** A store as listed, with what's in it and the store-wide discount in force. */
export type StoreSummary = ItemStore & {
  /** Items that aren't archived. */
  item_count: number;
  /** Sum of the active store-wide discounts, 0 to 100. */
  store_discount: number;
};

export type StoreWithServer = StoreSummary & { server: Server };

export type StoreDiscount = {
  id: number;
  store_id: number;
  /** Percent off, added to each item's own discount. */
  discount: number;
  message: string;
  created_at: string;
  last_updated: string | null;
  starts_at: string | null;
  /** Indefinite when `null`. */
  ends_at: string | null;
  /** Whether now is within `starts_at`/`ends_at`. */
  active: boolean;
};

export type StoreItem = {
  id: number;
  store_id: number;
  name: string;
  /** Plain text. */
  short_description: string;
  /** Markdown. */
  full_description: string;
  image_url: string | null;
  /** Ascending. */
  sort_order: number;
  created_at: string;
  last_updated: string | null;
  /** Listed but not buyable while false. */
  available: boolean;
  archived: boolean;
  /** Minor units of the store's currency (e.g. cents). */
  price: number;
  /** Percent off, before store-wide discounts. */
  discount: number;
  /** Limited releases; `null` is unlimited. */
  starting_stock: number | null;
  /** Excludes stock reserved by pending purchases. */
  remaining_stock: number | null;
  /** Purchases one user may make; `null` is unlimited. */
  max_per_user: number | null;
  all_time_purchases: number;
  rewards: Reward[];
};

/** An item with its price after every discount. */
export type PricedItem = StoreItem & {
  /** The item's discount plus the store's, capped at 100. */
  total_discount: number;
  /** Minor units, after `total_discount`. */
  final_price: number;
};

/** A store page, from `GET /store/{id}`. Only active discounts; items in display order. */
export type StorePage = StoreWithServer & {
  discounts: StoreDiscount[];
  items: PricedItem[];
};

/** Whether the current user may buy an item, from `GET /store/{id}/items/{id}`. */
export type Eligibility = {
  /** Paid and pending purchases of this item by the user. */
  purchased: number;
  can_buy: boolean;
  /** Why not, when `can_buy` is false. */
  reason: string | null;
};

export type ItemPage = {
  store: StoreSummary;
  server: Server;
  item: PricedItem;
  /** `null` when logged out. */
  buyer: Eligibility | null;
};

export type PurchaseStatus = "PENDING" | "PAID" | "CANCELLED";

/** A checkout attempt, from `POST /store/purchases/{id}/cancel`. */
export type Purchase = {
  id: number;
  store_id: number;
  item_id: number;
  buyer_id: number;
  steam_id: string;
  status: PurchaseStatus;
  stripe_checkout_session_id: string | null;
  stripe_payment_intent_id: string | null;
  quantity: number;
  list_price: number;
  discount_percentage: number;
  price: number;
  currency: string;
  rewards: Reward[];
  stock_reserved: boolean;
  created_at: string;
  completed_at: string | null;
};

/** The answer to `POST /store/{id}/items/{id}/checkout`. */
export type Checkout = {
  purchase_id: number;
  /** The Stripe Checkout page to send the buyer to. */
  url: string;
};

export type ReceiptStatus = "PAID" | "REFUNDED" | "DISPUTED";

export type FulfillmentStatus = "PENDING" | "FULFILLED" | "FAILED" | "REVOKED";

export type Receipt = {
  id: number;
  stripe_checkout_session_id: string;
  stripe_payment_intent_id: string;
  buyer_id: number;
  /** `null` once the item was deleted along with its server. */
  item_id: number | null;
  item_name: string;
  store_id: number | null;
  server_id: number | null;
  store_name: string;
  quantity: number;
  status: ReceiptStatus;
  purchased_at: string;
  /** Minor units, before discounts. */
  list_price: number;
  discount_percentage: number;
  /** Minor units, what was charged (before any tax). */
  price: number;
  currency: string;
  status_changed_at: string | null;
};

/** Delivery of a receipt's rewards in-game, done by the server's addon. */
export type Fulfillment = {
  id: number;
  receipt_id: number;
  buyer_id: number;
  steam_id: string;
  item_id: number | null;
  server_id: number;
  rewards: Reward[];
  status: FulfillmentStatus;
  attempts: number;
  last_error: string | null;
  created_at: string;
  fulfilled_at: string | null;
  revoked_at: string | null;
  revocation_applied_at: string | null;
};

/** A receipt with its buyer and delivery, from `GET /store/purchases` and `GET /admin/store/receipts`. */
export type ReceiptView = Receipt & {
  buyer: UserRef;
  fulfillment: Fulfillment | null;
};

/** Filters for `GET /admin/store/receipts`. Blank values are ignored. */
export type ReceiptFilters = {
  status?: ReceiptStatus | "";
  /** Store id. */
  store?: string;
  /** Buyer's user id. */
  buyer?: string;
};

/** A store's management page, from `GET /admin/store/{id}`: every discount and unarchived item. */
export type AdminStorePage = StoreWithServer & {
  discounts: StoreDiscount[];
  items: PricedItem[];
};

/** Sent to `POST /admin/store` (with `server_id`) and `PUT /admin/store/{id}`. */
export type StoreDetails = Pick<ItemStore, "enabled" | "currency" | "override_name" | "store_message">;

/** Sent to `POST /admin/store/{id}/items` and `PUT /admin/store/{id}/items/{id}`. */
export type ItemDetails = Pick<
  StoreItem,
  | "name"
  | "short_description"
  | "full_description"
  | "image_url"
  | "sort_order"
  | "available"
  | "price"
  | "discount"
  | "starting_stock"
  | "max_per_user"
  | "rewards"
>;

/** Sent to `POST /admin/store/{id}/discounts` and `PUT /admin/store/{id}/discounts/{id}`. */
export type DiscountDetails = Pick<StoreDiscount, "discount" | "message" | "starts_at" | "ends_at">;

// Storage (docs/STORAGE.md)

/** A file attached to a forum post or support message. See `fileUrl` in `@/lib/files`. */
export type Attachment = {
  /** The user whose folder it's in, or -1 for community files. */
  owner_id: number;
  id: string;
  name: string;
  content_type: string;
  /** Bytes. */
  size: number;
};

/** An uploaded file, from `POST /file/upload` and `POST /file/{owner_id}/{id}/complete`. */
export type StoredFile = Attachment & {
  /** Where to fetch it; stable across storage providers. */
  url: string;
  status: "PENDING" | "READY";
  uploaded_by: number | null;
  created_at: string;
  completed_at: string | null;
};

// Notifications (docs/NOTIFICATIONS.md)

export type NotificationType = "SUPPORT" | "STORE" | "FORUM" | "ACCOUNT" | "SYSTEM";

/** One of the current user's notifications. `title` and `body` are plain text: never render them as Markdown or HTML. */
export type Notification = {
  id: number;
  user_id: number;
  type: NotificationType;
  /** At most 128 characters. */
  title: string;
  /** At most 512 characters. */
  body: string | null;
  /** Site-relative path starting with a single "/", or `null` when there is nothing to link to (e.g. a deleted thread). */
  url: string | null;
  /** Bumped when a repeat collapses into this notification. */
  created_at: string;
  /** `null` while unread. */
  read_at: string | null;
};

/** A page of `GET /notifications`, newest first. `next_cursor` is `null` on the last page. */
export type NotificationPage = {
  notifications: Notification[];
  next_cursor: string | null;
};

/** The `/notifications` filters, from the URL. */
export type NotificationFilters = {
  unread: boolean;
  type: NotificationType | "";
};

/** Where and how to send a pending file's bytes. */
export type UploadTarget = {
  method: "PUT";
  url: string;
  /** Headers the request must carry, exactly as given. */
  headers: Record<string, string>;
  expires_at: string;
};
