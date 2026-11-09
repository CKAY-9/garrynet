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
