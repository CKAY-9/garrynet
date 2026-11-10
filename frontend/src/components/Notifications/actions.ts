"use server";

import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import { notificationTypes } from "@/lib/notificationEnums";
import type { Notification, NotificationType } from "@/lib/types";

/*
 * Notification writes. They don't refresh the page: the bell and the list keep their own
 * state and update it from the result (see `./events`). The backend only lets a user touch
 * their own notifications, so these need no checks beyond the login it already requires.
 */

const isId = (id: number) => Number.isSafeInteger(id) && id > 0;

/** Marks one notification read. Idempotent. */
export async function markNotificationRead(id: number): Promise<ActionResult<Notification>> {
  if (!isId(id)) return { ok: false, message: "Notification not found." };
  return runAction("mark the notification as read", async () => {
    const { data } = await backend.post<Notification>(`/notifications/${id}/read`, undefined, {
      headers: await authHeaders(),
    });
    return data;
  });
}

/** Marks every notification read, or only those of `type`. Returns how many were unread. */
export async function markAllNotificationsRead(type: NotificationType | ""): Promise<ActionResult<number>> {
  if (type && !notificationTypes.includes(type)) return { ok: false, message: "Unknown notification type." };
  return runAction("mark your notifications as read", async () => {
    const { data } = await backend.post<{ updated: number }>("/notifications/read-all", type ? { type } : undefined, {
      headers: await authHeaders(),
    });
    return data.updated;
  });
}

/** Deletes one notification. */
export async function deleteNotification(id: number): Promise<ActionResult<null>> {
  if (!isId(id)) return { ok: false, message: "Notification not found." };
  return runAction("delete the notification", async () => {
    await backend.delete(`/notifications/${id}`, { headers: await authHeaders() });
    return null;
  });
}

/** Deletes every read notification, of every type. Returns how many were deleted. */
export async function deleteReadNotifications(): Promise<ActionResult<number>> {
  return runAction("delete your read notifications", async () => {
    const { data } = await backend.delete<{ deleted: number }>("/notifications", { headers: await authHeaders() });
    return data.deleted;
  });
}
