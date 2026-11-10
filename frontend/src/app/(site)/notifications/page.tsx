import { Suspense } from "react";
import { unauthorized } from "next/navigation";
import { NotificationFilterNav, NotificationList } from "@/components/Notifications";
import { getCurrentUser } from "@/lib/auth";
import { getNotifications, readNotificationFilters, type SearchParams } from "@/lib/notifications";
import styles from "@/components/Notifications/Notifications.module.scss";

export default function NotificationsPage({ searchParams }: PageProps<"/notifications">) {
  return (
    // The session and notifications are request-time, so they stream in behind the boundary.
    <Suspense>
      <Notifications searchParams={searchParams} />
    </Suspense>
  );
}

/** /notifications: every notification, filterable by read state and type. Login required. */
async function Notifications({ searchParams }: { searchParams: SearchParams }) {
  const [user, filters] = await Promise.all([getCurrentUser(), readNotificationFilters(searchParams)]);
  if (!user) unauthorized();
  const page = await getNotifications(filters);
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1>Notifications</h1>
        <p>Replies, purchases and account changes that concern you. Read ones are removed after 30 days.</p>
      </header>
      <NotificationFilterNav filters={filters} />
      <NotificationList key={`${filters.unread}-${filters.type}`} initial={page} filters={filters} />
    </div>
  );
}
