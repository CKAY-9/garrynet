"use server";

import { redirect } from "next/navigation";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { PrivateMessage } from "@/lib/types";

/*
 * Private message writes (docs/PRIVATE_MESSAGES.md). Sending and marking read don't refresh
 * the page: the open chat keeps its own state and updates it from the result, and announces
 * the change so the navigation's unread badge follows (see `./events`). The backend checks
 * permissions, the recipient and the rate limit.
 */

const isId = (id: number) => Number.isSafeInteger(id) && id > 0;

/** Sends a message with the uploaded files `attachments` and returns it. */
export async function sendMessage(
  userId: number,
  content: string,
  attachments: string[] = [],
  temporary = false,
): Promise<ActionResult<PrivateMessage>> {
  if (!isId(userId)) return { ok: false, message: "User not found." };
  return runAction(
    "send the message",
    async () => {
      const { data } = await backend.post<PrivateMessage>(
        `/messages/with/${userId}`,
        { content, attachments, temporary },
        { headers: await authHeaders() },
      );
      return data;
    },
    // The backend doesn't say whether the sender or the recipient lacks the permission.
    { 403: "You can't message this user right now." },
  );
}

/** Marks what user `userId` sent the current user as read. Returns how many were unread. */
export async function markChatRead(userId: number): Promise<ActionResult<number>> {
  if (!isId(userId)) return { ok: false, message: "User not found." };
  return runAction("mark the chat as read", async () => {
    const { data } = await backend.post<{ updated: number }>(`/messages/with/${userId}/read`, undefined, {
      headers: await authHeaders(),
    });
    return data.updated;
  });
}

/** Deletes the chat with user `userId` for the current user only, then opens the chat list. */
export async function deleteChat(userId: number): Promise<ActionResult<never>> {
  if (!isId(userId)) return { ok: false, message: "User not found." };
  const result = await runAction("delete the chat", async () => {
    await backend.delete(`/messages/with/${userId}`, { headers: await authHeaders() });
  });
  if (!result.ok) return result;
  redirect("/messages");
}
