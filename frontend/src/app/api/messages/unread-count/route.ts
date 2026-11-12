import { getUnreadMessageCount } from "@/lib/messages";
import { jsonResponse } from "@/lib/notifications";

/**
 * `{ count }` of the current user's unread messages, for the navigation badge's 60-second
 * poll. The browser can't call the backend (httpOnly cookie, no CORS), so this proxies it,
 * as a GET handler rather than a server action for the reasons given in
 * `app/api/notifications/unread-count`.
 */
export async function GET() {
  return jsonResponse(async () => ({ count: await getUnreadMessageCount() }), "unread messages");
}
