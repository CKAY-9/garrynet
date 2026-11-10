import type { Notification, NotificationType } from "@/lib/types";

/*
 * Keeps the bell, its dropdown and the `/notifications` page in step without waiting for the
 * next poll. Whoever changes notifications announces it; the bell refetches the unread
 * count, and any open list applies the change to the rows it has loaded.
 */

export type NotificationChange =
  | { kind: "read"; ids: number[] }
  | { kind: "deleted"; ids: number[] }
  /** Every notification, or every one of `type`, is now read. */
  | { kind: "allRead"; type: NotificationType | "" }
  | { kind: "readDeleted" };

const EVENT = "garrynet:notifications";

export function announceChange(change: NotificationChange) {
  window.dispatchEvent(new CustomEvent<NotificationChange>(EVENT, { detail: change }));
}

/** Calls `listener` on every announced change. Returns the unsubscribe function. */
export function subscribeToChanges(listener: (change: NotificationChange) => void): () => void {
  const handler = (e: Event) => listener((e as CustomEvent<NotificationChange>).detail);
  window.addEventListener(EVENT, handler);
  return () => window.removeEventListener(EVENT, handler);
}

/** `list` after `change`. Idempotent, so applying a change twice is harmless. */
export function applyChange(list: Notification[], change: NotificationChange): Notification[] {
  const now = new Date().toISOString();
  const markRead = (n: Notification) => (n.read_at ? n : { ...n, read_at: now });
  switch (change.kind) {
    case "read":
      return list.map((n) => (change.ids.includes(n.id) ? markRead(n) : n));
    case "deleted":
      return list.filter((n) => !change.ids.includes(n.id));
    case "allRead":
      return list.map((n) => (!change.type || n.type === change.type ? markRead(n) : n));
    case "readDeleted":
      return list.filter((n) => !n.read_at);
  }
}

/**
 * `list` with `page` appended, newest first. A repeat can collapse into an existing row and
 * move it, so a row may come back in a later page: keep one copy, with the newest data.
 */
export function mergePage(list: Notification[], page: Notification[]): Notification[] {
  const incoming = new Map(page.map((n) => [n.id, n]));
  const merged = list.map((n) => incoming.get(n.id) ?? n);
  const seen = new Set(list.map((n) => n.id));
  return [...merged, ...page.filter((n) => !seen.has(n.id))];
}
