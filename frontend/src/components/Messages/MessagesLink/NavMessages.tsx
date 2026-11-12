import { getCurrentUser } from "@/lib/auth";
import { MESSAGING } from "@/lib/messageEnums";
import { getUnreadMessageCount } from "@/lib/messages";
import { hasPermission } from "@/lib/permissions";
import { MessagesLink } from "./MessagesLink";

/**
 * The "Messages" link for the side navigation, for users who may use messages. Reads the
 * session cookie, so render it inside a `<Suspense>` boundary.
 */
export async function NavMessages() {
  const user = await getCurrentUser();
  if (!user || !hasPermission(user.permissions, MESSAGING)) return null;
  let count = 0;
  try {
    count = await getUnreadMessageCount();
  } catch (err) {
    // The link still works; its first poll fills the count in.
    console.error("Failed to fetch unread messages:", err instanceof Error ? err.message : err);
  }
  return <MessagesLink initialCount={count} />;
}
