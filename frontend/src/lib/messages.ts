import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { authHeaders } from "./auth";
import { backend } from "./backend";
import { PAGE_SIZE, pageParams } from "./messageEnums";
import type { ChatPage, MessageThread } from "./types";

/*
 * Private message reads (docs/PRIVATE_MESSAGES.md). All request-time, so render anything that
 * uses these inside a `<Suspense>` boundary. The browser can't reach the backend, so loading
 * more and polling go through the route handlers in `app/api/messages`, and writes are server
 * actions in `@/components/Messages/actions`. The backend requires Site Access and User
 * Messages for all of them.
 */

const isUserId = (id: string) => /^\d{1,18}$/.test(id);

/** A page of the current user's chats, most recent first, with whether they have ever sent a message. */
export async function getChats(cursor?: string | null, limit = PAGE_SIZE): Promise<ChatPage> {
  await connection();
  const { data } = await backend.get<ChatPage>(`/messages/chats?${pageParams(cursor, limit)}`, {
    headers: await authHeaders(),
  });
  return data;
}

/** Whether the current user has ever sent a message (the cheapest read that says so). */
export async function hasSentMessage(): Promise<boolean> {
  return (await getChats(null, 1)).has_sent;
}

/**
 * A page of the current user's messages with user `userId`, newest first. Throws on any
 * failure, including 404 for an unknown user; see `getThread` for pages.
 */
export async function readThread(userId: string, cursor?: string | null, limit = PAGE_SIZE): Promise<MessageThread> {
  await connection();
  const { data } = await backend.get<MessageThread>(`/messages/with/${userId}?${pageParams(cursor, limit)}`, {
    headers: await authHeaders(),
  });
  return data;
}

/** The most recent messages with user `id`, or `null` if there is no such user. */
export async function getThread(id: string): Promise<MessageThread | null> {
  if (!isUserId(id)) return null;
  try {
    return await readThread(id);
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

/** How many messages the current user hasn't read, across every chat. */
export async function getUnreadMessageCount(): Promise<number> {
  await connection();
  const { data } = await backend.get<{ count: number }>("/messages/unread-count", { headers: await authHeaders() });
  return data.count;
}
