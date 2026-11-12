import type { NextRequest } from "next/server";
import { PAGE_SIZE, parseLimit } from "@/lib/messageEnums";
import { getChats } from "@/lib/messages";
import { jsonResponse } from "@/lib/notifications";

/**
 * A page of the current user's chats (`{ items, next_cursor, has_sent }`), for the chat
 * list's "Load more". Query: `limit` and `cursor`, as on the backend. Proxied for the same
 * reason as `unread-count`.
 */
export async function GET(request: NextRequest) {
  const query = request.nextUrl.searchParams;
  return jsonResponse(() => getChats(query.get("cursor"), parseLimit(query.get("limit")) ?? PAGE_SIZE), "chats");
}
