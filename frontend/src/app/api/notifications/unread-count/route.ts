import { getUnreadCount, jsonResponse } from "@/lib/notifications";

/**
 * `{ count }` of the current user's unread notifications, for the bell's 60-second poll.
 * The browser can't call the backend (httpOnly cookie, no CORS), so this proxies it. A GET
 * handler rather than a server action: actions are dispatched one at a time per client and
 * are meant for mutations, so a poll would queue in front of the user's real actions.
 */
export async function GET() {
  return jsonResponse(async () => ({ count: await getUnreadCount() }));
}
