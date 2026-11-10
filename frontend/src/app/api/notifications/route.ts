import type { NextRequest } from "next/server";
import { getNotifications, jsonResponse, parseNotificationType } from "@/lib/notifications";
import { PAGE_SIZE } from "@/lib/notificationEnums";

/**
 * A page of the current user's notifications (`{ notifications, next_cursor }`), for the
 * bell's dropdown and the "Load more" button. Query: `unread=true`, `type`, `limit` and
 * `cursor`, as on the backend. Proxied for the same reason as `unread-count`.
 */
export async function GET(request: NextRequest) {
  const query = request.nextUrl.searchParams;
  const limit = Number(query.get("limit"));
  return jsonResponse(() =>
    getNotifications(
      { unread: query.get("unread") === "true", type: parseNotificationType(query.get("type")) },
      query.get("cursor"),
      Number.isInteger(limit) && limit >= 1 && limit <= 100 ? limit : PAGE_SIZE,
    ),
  );
}
