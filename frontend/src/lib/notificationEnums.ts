import type { NotificationFilters, NotificationType } from "./types";

/*
 * Notification types (docs/NOTIFICATIONS.md) in display order, with their labels, plus the
 * small helpers the bell and the page share. Safe to import from client components.
 */

export const notificationTypes: NotificationType[] = ["SUPPORT", "STORE", "FORUM", "ACCOUNT", "SYSTEM"];

export const notificationTypeLabels: Record<NotificationType, string> = {
  SUPPORT: "Support",
  STORE: "Store",
  FORUM: "Forum",
  ACCOUNT: "Account",
  SYSTEM: "System",
};

/** How many notifications the bell's dropdown shows. */
export const DROPDOWN_SIZE = 5;
/** How many notifications `/notifications` loads at a time (the backend allows 1 to 100). */
export const PAGE_SIZE = 25;
/** How often the bell refreshes the unread count while the tab is visible. */
export const POLL_INTERVAL_MS = 60_000;

/** The badge text: the count, or "99+" above 99. */
export function badgeText(count: number): string {
  return count > 99 ? "99+" : String(count);
}

/** Query string for `GET /notifications` (and the `/api/notifications` proxy). Blank values are left out. */
export function notificationParams(filters: NotificationFilters, cursor?: string | null, limit?: number): URLSearchParams {
  const params = new URLSearchParams();
  if (filters.unread) params.set("unread", "true");
  if (filters.type) params.set("type", filters.type);
  if (limit) params.set("limit", String(limit));
  if (cursor) params.set("cursor", cursor);
  return params;
}

/** The `/notifications` URL for a set of filters. */
export function notificationsHref(filters: NotificationFilters): string {
  const query = notificationParams(filters).toString();
  return query ? `/notifications?${query}` : "/notifications";
}

/** A notification's link, or `null`. The backend only stores site-relative paths; this is a second guard. */
export function notificationHref(url: string | null): string | null {
  return url && url.startsWith("/") && !url.startsWith("//") ? url : null;
}
