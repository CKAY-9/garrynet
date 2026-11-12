import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { authHeaders } from "./auth";
import { backend } from "./backend";
import { PAGE_SIZE, pageParams } from "./messageEnums";
import type { AdminChatPage, AdminMessageThread } from "./types";

/*
 * Staff reads of anyone's private messages (docs/PRIVATE_MESSAGES.md#admins), for
 * `/admin/messages`. Request-time, so render anything that uses these inside a `<Suspense>`
 * boundary. "Load more" goes through the route handlers in `app/api/admin/messages`. The
 * backend requires Admin Access and User Management and logs every read.
 */

const isUserId = (id: string) => /^\d{1,18}$/.test(id);

/** `null` for a 404 (an unknown user); rethrows anything else. */
async function orNotFound<T>(read: () => Promise<T>): Promise<T | null> {
  try {
    return await read();
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

/** A page of user `userId`'s chats, most recent first. Throws on any failure, including 404. */
export async function readAdminChats(userId: string, cursor?: string | null, limit = PAGE_SIZE): Promise<AdminChatPage> {
  await connection();
  const { data } = await backend.get<AdminChatPage>(`/messages/admin/${userId}/chats?${pageParams(cursor, limit)}`, {
    headers: await authHeaders(),
  });
  return data;
}

/** The first page of user `id`'s chats, or `null` if there is no such user. */
export async function getAdminChats(id: string): Promise<AdminChatPage | null> {
  if (!isUserId(id)) return null;
  return orNotFound(() => readAdminChats(id));
}

/**
 * A page of the messages between users `userId` and `otherId`, newest first. Throws on any
 * failure, including 404 when either user doesn't exist.
 */
export async function readAdminThread(
  userId: string,
  otherId: string,
  cursor?: string | null,
  limit = PAGE_SIZE,
): Promise<AdminMessageThread> {
  await connection();
  const { data } = await backend.get<AdminMessageThread>(
    `/messages/admin/${userId}/with/${otherId}?${pageParams(cursor, limit)}`,
    { headers: await authHeaders() },
  );
  return data;
}

/** The most recent messages between users `userId` and `otherId`, or `null` if either doesn't exist. */
export async function getAdminThread(userId: string, otherId: string): Promise<AdminMessageThread | null> {
  if (!isUserId(userId) || !isUserId(otherId)) return null;
  return orNotFound(() => readAdminThread(userId, otherId));
}
