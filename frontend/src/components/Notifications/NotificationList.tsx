"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { ActionResult } from "@/lib/actionResult";
import { notificationHref, notificationParams, notificationTypeLabels, PAGE_SIZE } from "@/lib/notificationEnums";
import type { Notification, NotificationFilters, NotificationPage } from "@/lib/types";
import { deleteNotification, deleteReadNotifications, markAllNotificationsRead, markNotificationRead } from "./actions";
import { announceChange, applyChange, mergePage, subscribeToChanges, type NotificationChange } from "./events";
import { Ago, TypeIcon } from "./parts";
import styles from "./Notifications.module.scss";

type Status = { error: boolean; text: string } | null;

/**
 * The `/notifications` list for one set of filters: the first page from the server, then
 * "Load more". Keyed by the filters, so changing them starts a fresh list. Every change is
 * announced, which updates the rows here and the bell's count.
 */
export function NotificationList({ initial, filters }: { initial: NotificationPage; filters: NotificationFilters }) {
  const [items, setItems] = useState(initial.notifications);
  const [cursor, setCursor] = useState(initial.next_cursor);
  const [loadingMore, setLoadingMore] = useState(false);
  /** What is in flight: "readAll", "deleteRead", or "read-<id>" / "delete-<id>". */
  const [busy, setBusy] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>(null);
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  // Applies changes from here and from the bell's dropdown.
  useEffect(() => subscribeToChanges((change) => setItems((list) => applyChange(list, change))), []);

  // The Unread view drops rows once they are read.
  const visible = filters.unread ? items.filter((n) => !n.read_at) : items;
  const typeLabel = filters.type ? notificationTypeLabels[filters.type] : "";

  async function run<T>(
    key: string,
    action: () => Promise<ActionResult<T>>,
    onDone: (data: T) => { change: NotificationChange; message?: string },
  ) {
    setBusy(key);
    setStatus(null);
    const res = await action();
    setBusy(null);
    if (!res.ok) {
      setStatus({ error: true, text: res.message });
      return;
    }
    const { change, message } = onDone(res.data);
    announceChange(change);
    if (message) setStatus({ error: false, text: message });
  }

  const markRead = (n: Notification) =>
    run(`read-${n.id}`, () => markNotificationRead(n.id), () => ({ change: { kind: "read", ids: [n.id] } }));

  const remove = (n: Notification) =>
    run(`delete-${n.id}`, () => deleteNotification(n.id), () => ({ change: { kind: "deleted", ids: [n.id] } }));

  const markAllRead = () =>
    run("readAll", () => markAllNotificationsRead(filters.type), (updated) => {
      // Nothing unread of this kind is left to load.
      if (filters.unread) setCursor(null);
      return {
        change: { kind: "allRead", type: filters.type },
        message: updated === 0 ? "Nothing was unread." : `Marked ${plural(updated)} as read.`,
      };
    });

  const deleteRead = () =>
    run("deleteRead", deleteReadNotifications, (deleted) => {
      setConfirmingDelete(false);
      return {
        change: { kind: "readDeleted" },
        message: deleted === 0 ? "There were no read notifications to delete." : `Deleted ${plural(deleted)}.`,
      };
    });

  /** Following a row's link marks it read; navigation doesn't wait for it. */
  function follow(n: Notification) {
    if (n.read_at) return;
    void markNotificationRead(n.id).then((res) => {
      if (res.ok) announceChange({ kind: "read", ids: [n.id] });
    });
  }

  async function loadMore() {
    if (!cursor) return;
    setLoadingMore(true);
    setStatus(null);
    let failure = "Couldn't load more notifications. Try again.";
    try {
      const res = await fetch(`/api/notifications?${notificationParams(filters, cursor, PAGE_SIZE)}`, {
        cache: "no-store",
      });
      if (res.ok) {
        const page = (await res.json()) as NotificationPage;
        setItems((list) => mergePage(list, page.notifications));
        setCursor(page.next_cursor);
        return;
      }
      if (res.status === 401) failure = "Your session has expired. Log in again to see more notifications.";
    } catch {
      // Network failure; keep the generic message.
    } finally {
      setLoadingMore(false);
    }
    setStatus({ error: true, text: failure });
  }

  const empty = filters.unread
    ? typeLabel
      ? `No unread ${typeLabel.toLowerCase()} notifications.`
      : "You're all caught up."
    : typeLabel
      ? `No ${typeLabel.toLowerCase()} notifications.`
      : "No notifications yet.";

  return (
    <section className={styles.listSection} aria-label="Notifications">
      <div className={styles.bulk}>
        <button type="button" className={styles.tool} onClick={markAllRead} disabled={busy !== null}>
          {busy === "readAll" ? "Marking..." : typeLabel ? `Mark ${typeLabel.toLowerCase()} notifications as read` : "Mark all as read"}
        </button>
        {confirmingDelete ? (
          <span className={styles.confirm} role="group" aria-label="Confirm deleting read notifications">
            <span>
              {typeLabel
                ? `Delete every read notification, of all types (not only ${typeLabel.toLowerCase()})?`
                : "Delete every read notification?"}
            </span>
            <button type="button" className={`${styles.tool} ${styles.dangerTool}`} onClick={deleteRead} disabled={busy !== null}>
              {busy === "deleteRead" ? "Deleting..." : "Delete"}
            </button>
            <button type="button" className={styles.tool} onClick={() => setConfirmingDelete(false)} disabled={busy !== null}>
              Cancel
            </button>
          </span>
        ) : (
          <button type="button" className={styles.tool} onClick={() => setConfirmingDelete(true)} disabled={busy !== null}>
            {typeLabel ? "Delete all read (every type)" : "Delete read"}
          </button>
        )}
      </div>
      <p className={`${styles.status} ${status?.error ? styles.error : ""}`} role="status">
        {status?.text}
      </p>
      <ul className={styles.rows}>
        {visible.map((n) => (
          <Row
            key={n.id}
            notification={n}
            busy={busy}
            onFollow={follow}
            onMarkRead={markRead}
            onDelete={remove}
          />
        ))}
        {visible.length === 0 && !cursor && <li className={styles.empty}>{empty}</li>}
      </ul>
      {cursor && (
        <button type="button" className={`${styles.tool} ${styles.loadMore}`} onClick={loadMore} disabled={loadingMore}>
          {loadingMore ? "Loading..." : "Load more"}
        </button>
      )}
    </section>
  );
}

function Row({
  notification: n,
  busy,
  onFollow,
  onMarkRead,
  onDelete,
}: {
  notification: Notification;
  busy: string | null;
  onFollow: (n: Notification) => void;
  onMarkRead: (n: Notification) => void;
  onDelete: (n: Notification) => void;
}) {
  const href = notificationHref(n.url);
  const unread = !n.read_at;
  return (
    <li className={`${styles.row} ${unread ? styles.unreadRow : ""}`}>
      <TypeIcon type={n.type} className={styles.typeIcon} />
      <div className={styles.rowMain}>
        <span className={`${styles.rowTitle} ${unread ? styles.unreadTitle : ""}`}>
          {/* The link stretches over the whole row (see `.rowLink`). */}
          {href ? (
            <Link className={styles.rowLink} href={href} onClick={() => onFollow(n)}>
              {n.title}
            </Link>
          ) : (
            n.title
          )}
          {unread && <span className={styles.srOnly}> (unread)</span>}
        </span>
        {n.body && <p className={styles.rowBody}>{n.body}</p>}
        <span className={styles.meta}>
          {notificationTypeLabels[n.type]} · <Ago iso={n.created_at} />
        </span>
      </div>
      <div className={styles.rowTools}>
        {unread && (
          <button type="button" className={styles.tool} onClick={() => onMarkRead(n)} disabled={busy !== null}>
            {busy === `read-${n.id}` ? "Marking..." : "Mark read"}
          </button>
        )}
        <button
          type="button"
          className={`${styles.tool} ${styles.dangerTool}`}
          onClick={() => onDelete(n)}
          disabled={busy !== null}
          aria-label={`Delete "${n.title}"`}
        >
          {busy === `delete-${n.id}` ? "Deleting..." : "Delete"}
        </button>
      </div>
    </li>
  );
}

function plural(n: number): string {
  return `${n.toLocaleString("en")} ${n === 1 ? "notification" : "notifications"}`;
}
