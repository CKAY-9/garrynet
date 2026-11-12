import type { NextRequest } from "next/server";
import { readAdminThread } from "@/lib/adminMessages";
import { PAGE_SIZE, parseLimit } from "@/lib/messageEnums";
import { jsonResponse } from "@/lib/notifications";

/**
 * Staff: a page of the messages between users `user_id` and `other_id`, newest first
 * (`{ user, other, items, next_cursor }`), for loading older messages on
 * `/admin/messages/{user_id}/{other_id}`. Query: `limit` and `cursor`, as on the backend,
 * which checks Admin Access and User Management and logs the read. Proxied because the
 * browser can't reach the backend.
 */
export async function GET(request: NextRequest, ctx: RouteContext<"/api/admin/messages/[user_id]/[other_id]">) {
  const { user_id, other_id } = await ctx.params;
  if (!/^\d{1,18}$/.test(user_id) || !/^\d{1,18}$/.test(other_id)) {
    return Response.json({ error: "user not found" }, { status: 404, headers: { "Cache-Control": "private, no-store" } });
  }
  const query = request.nextUrl.searchParams;
  return jsonResponse(
    () => readAdminThread(user_id, other_id, query.get("cursor"), parseLimit(query.get("limit")) ?? PAGE_SIZE),
    "messages",
  );
}
