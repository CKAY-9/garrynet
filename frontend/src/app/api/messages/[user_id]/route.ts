import type { NextRequest } from "next/server";
import { PAGE_SIZE, parseLimit } from "@/lib/messageEnums";
import { readThread } from "@/lib/messages";
import { jsonResponse } from "@/lib/notifications";

/**
 * A page of the current user's messages with user `user_id`, newest first
 * (`{ user, items, next_cursor }`): older pages as the chat scrolls up (with `cursor`), and
 * the newest page for its 15-second poll. Query: `limit` and `cursor`, as on the backend.
 * Proxied for the same reason as `unread-count`.
 */
export async function GET(request: NextRequest, ctx: RouteContext<"/api/messages/[user_id]">) {
  const { user_id } = await ctx.params;
  if (!/^\d{1,18}$/.test(user_id)) {
    return Response.json({ error: "user not found" }, { status: 404, headers: { "Cache-Control": "private, no-store" } });
  }
  const query = request.nextUrl.searchParams;
  return jsonResponse(
    () => readThread(user_id, query.get("cursor"), parseLimit(query.get("limit")) ?? PAGE_SIZE),
    "messages",
  );
}
