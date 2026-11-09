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
