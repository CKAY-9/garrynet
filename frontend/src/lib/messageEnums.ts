import { Permission } from "./permissions";

/*
 * Private message limits and helpers (docs/PRIVATE_MESSAGES.md). Safe to import from client
 * components.
 */

/**
 * What sending and reading messages takes, as on the backend (`SEND` in
 * src/routes/messages.rs). Suspensions and bans remove `User Messages`.
 */
export const MESSAGING = Permission.SiteAccess | Permission.UserMessages;

/** What reading anyone's messages at `/admin/messages` takes, as on the backend (`VIEW_ANY`). */
export const READ_ANY_MESSAGES = Permission.AdminAccess | Permission.UserManagement;

/** Limits enforced by the backend (src/routes/messages.rs). */
export const MAX_CONTENT = 5_000;
/** Messages and chats loaded at a time (the backend allows 1 to 100). */
export const PAGE_SIZE = 25;
/** How often an open chat checks for new messages while the tab is visible. */
export const THREAD_POLL_INTERVAL_MS = 15_000;
/** How often the navigation refreshes the unread count while the tab is visible. */
export const UNREAD_POLL_INTERVAL_MS = 60_000;

/** Query string for the paged reads (and their `/api/messages` proxies). Blank values are left out. */
export function pageParams(cursor?: string | null, limit?: number): URLSearchParams {
  const params = new URLSearchParams();
  if (limit) params.set("limit", String(limit));
  if (cursor) params.set("cursor", cursor);
  return params;
}

/** A `limit` query value, if it's one the backend accepts. */
export function parseLimit(value: string | null): number | undefined {
  const limit = Number(value);
  return value && Number.isInteger(limit) && limit >= 1 && limit <= 100 ? limit : undefined;
}

/** The chat with a user. */
export const chatHref = (userId: number) => `/messages/${userId}`;

/** Staff: a user's chats. */
export const adminChatsHref = (userId: number) => `/admin/messages/${userId}`;
