import type { NextRequest } from "next/server";
import { readAdminChats } from "@/lib/adminMessages";
import { PAGE_SIZE, parseLimit } from "@/lib/messageEnums";
import { jsonResponse } from "@/lib/notifications";

/**
 * Staff: a page of user `user_id`'s chats (`{ user, items, next_cursor }`), for the
 * `/admin/messages/{user_id}` "Load more". Query: `limit` and `cursor`, as on the backend,
 * which checks Admin Access and User Management and logs the read. Proxied because the
 * browser can't reach the backend.
 */
export async function GET(request: NextRequest, ctx: RouteContext<"/api/admin/messages/[user_id]">) {
  const { user_id } = await ctx.params;
  if (!/^\d{1,18}$/.test(user_id)) {
    return Response.json({ error: "user not found" }, { status: 404, headers: { "Cache-Control": "private, no-store" } });
  }
  const query = request.nextUrl.searchParams;
  return jsonResponse(
    () => readAdminChats(user_id, query.get("cursor"), parseLimit(query.get("limit")) ?? PAGE_SIZE),
    "chats",
  );
}
