import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { authHeaders } from "./auth";
import { backend } from "./backend";
import { notificationParams, notificationTypes, PAGE_SIZE } from "./notificationEnums";
import type { NotificationFilters, NotificationPage, NotificationType } from "./types";

/*
 * Notification reads (docs/NOTIFICATIONS.md). All request-time, so render anything that uses
 * these inside a `<Suspense>` boundary. The browser can't reach the backend, so the bell and
 * "Load more" read through the route handlers in `app/api/notifications`, and writes are
 * server actions in `@/components/Notifications/actions`.
 */

/** A page's `searchParams`. */
export type SearchParams = Promise<Record<string, string | string[] | undefined>>;

function firstParam(params: Record<string, string | string[] | undefined>, key: string): string {
  const value = params[key];
  return (Array.isArray(value) ? value[0] : value)?.trim() ?? "";
}

/** A notification type, or `""` if `value` isn't one. */
export function parseNotificationType(value: string | null | undefined): NotificationType | "" {
  return notificationTypes.includes(value as NotificationType) ? (value as NotificationType) : "";
}

/** The `/notifications` filters from the URL; unknown values are dropped. */
export async function readNotificationFilters(searchParams: SearchParams): Promise<NotificationFilters> {
  const params = await searchParams;
  return {
    unread: firstParam(params, "unread") === "true",
    type: parseNotificationType(firstParam(params, "type")),
  };
}

/** A page of the current user's notifications, newest first. */
export async function getNotifications(
  filters: NotificationFilters,
  cursor?: string | null,
  limit = PAGE_SIZE,
): Promise<NotificationPage> {
  await connection();
  const { data } = await backend.get<NotificationPage>(`/notifications?${notificationParams(filters, cursor, limit)}`, {
    headers: await authHeaders(),
  });
  return data;
}

/** How many of the current user's notifications are unread (uncapped). */
export async function getUnreadCount(): Promise<number> {
  await connection();
  const { data } = await backend.get<{ count: number }>("/notifications/unread-count", { headers: await authHeaders() });
  return data.count;
}

/**
 * Runs a read for a route handler and answers with its JSON, or with the backend's status
 * and `{ error }` when it fails (401 when logged out). Never cached: it is per user.
 */
export async function jsonResponse(read: () => Promise<unknown>): Promise<Response> {
  const headers = { "Cache-Control": "private, no-store" };
  try {
    return Response.json(await read(), { headers });
  } catch (err) {
    const status = axios.isAxiosError(err) ? err.response?.status : undefined;
    if (status) {
      const error = axios.isAxiosError(err) ? err.response?.data?.error : undefined;
      return Response.json({ error: typeof error === "string" ? error : "request failed" }, { status, headers });
    }
    // Log only the message: the axios error carries the request config, including the token.
    console.error("Failed to read notifications:", err instanceof Error ? err.message : err);
    return Response.json({ error: "couldn't reach the server" }, { status: 502, headers });
  }
}
