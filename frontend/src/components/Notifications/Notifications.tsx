import Link from "next/link";
import { getCurrentUser } from "@/lib/auth";
import { notificationsHref, notificationTypeLabels, notificationTypes } from "@/lib/notificationEnums";
import { getUnreadCount } from "@/lib/notifications";
import type { NotificationFilters } from "@/lib/types";
import { NotificationBell } from "./NotificationBell";
import styles from "./Notifications.module.scss";

/**
 * The bell for the side navigation, for logged-in users only. Reads the session cookie, so
 * render it inside a `<Suspense>` boundary.
 */
export async function NavNotifications() {
  const user = await getCurrentUser();
  if (!user) return null;
  let count = 0;
  try {
    count = await getUnreadCount();
  } catch (err) {
    // The bell still works; its first poll fills the count in.
    console.error("Failed to fetch unread notifications:", err instanceof Error ? err.message : err);
  }
  return <NotificationBell initialCount={count} />;
}

/** All / Unread tabs and the type filter for `/notifications`. Plain links: the filters live in the URL. */
export function NotificationFilterNav({ filters }: { filters: NotificationFilters }) {
  const tabs = [
    { unread: false, label: "All" },
    { unread: true, label: "Unread" },
  ];
  const types = [{ type: "" as const, label: "All types" }, ...notificationTypes.map((type) => ({ type, label: notificationTypeLabels[type] }))];
  return (
    <div className={styles.filters}>
      <nav aria-label="Read state">
        <ul className={styles.tabs}>
          {tabs.map((tab) => {
            const active = tab.unread === filters.unread;
            return (
              <li key={tab.label}>
                <Link
                  className={`${styles.tab} ${active ? styles.activeTab : ""}`}
                  href={notificationsHref({ ...filters, unread: tab.unread })}
                  aria-current={active ? "page" : undefined}
                >
                  {tab.label}
                </Link>
              </li>
            );
          })}
        </ul>
      </nav>
      <nav aria-label="Notification type">
        <ul className={styles.typeChips}>
          {types.map(({ type, label }) => {
            const active = type === filters.type;
            return (
              <li key={label}>
                <Link
                  className={`${styles.typeChip} ${active ? styles.activeChip : ""}`}
                  href={notificationsHref({ ...filters, type })}
                  aria-current={active ? "page" : undefined}
                >
                  {label}
                </Link>
              </li>
            );
          })}
        </ul>
      </nav>
    </div>
  );
}
