"use client";

import Link from "next/link";
import { useCallback, useEffect, useId, useRef, useState, type CSSProperties } from "react";
import { BellIcon } from "@/components/SideNav/icons";
import { badgeText, DROPDOWN_SIZE, notificationHref, POLL_INTERVAL_MS } from "@/lib/notificationEnums";
import type { Notification, NotificationPage } from "@/lib/types";
import { markNotificationRead } from "./actions";
import { announceChange, applyChange, subscribeToChanges } from "./events";
import { Ago, TypeIcon } from "./parts";
import nav from "@/components/SideNav/SideNav.module.scss";
import styles from "./Notifications.module.scss";

/**
 * The bell in the side navigation: the unread count, refreshed every minute while the tab
 * is visible, and a dropdown of the most recent notifications. `initialCount` comes from
 * the server render, which counts as the page-load fetch.
 */
export function NotificationBell({ initialCount }: { initialCount: number }) {
  const [count, setCount] = useState(initialCount);
  const [open, setOpen] = useState(false);
  const [recent, setRecent] = useState<Notification[] | null>(null);
  const [failed, setFailed] = useState(false);
  const [position, setPosition] = useState<CSSProperties>();
  const wrapper = useRef<HTMLDivElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const panelId = useId();

  const refreshCount = useCallback(async () => {
    try {
      const res = await fetch("/api/notifications/unread-count", { cache: "no-store" });
      // Logged out in another tab, or the session expired.
      if (res.status === 401) setCount(0);
      else if (res.ok) setCount(((await res.json()) as { count: number }).count);
    } catch {
      // Offline or the server is restarting; the next poll tries again.
    }
  }, []);

  const loadRecent = useCallback(async () => {
    setFailed(false);
    try {
      const res = await fetch(`/api/notifications?limit=${DROPDOWN_SIZE}`, { cache: "no-store" });
      if (!res.ok) throw new Error(`status ${res.status}`);
      setRecent(((await res.json()) as NotificationPage).notifications);
    } catch {
      setFailed(true);
    }
  }, []);

  // Poll while the tab is visible. Every backend request marks the user online, so a hidden
  // tab must not keep polling; coming back refetches at once and restarts the timer.
  useEffect(() => {
    let timer: number | undefined;
    const stop = () => {
      window.clearInterval(timer);
      timer = undefined;
    };
    const start = () => {
      stop();
      timer = window.setInterval(refreshCount, POLL_INTERVAL_MS);
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void refreshCount();
        start();
      } else {
        stop();
      }
    };
    if (document.visibilityState === "visible") start();
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [refreshCount]);

  // Changes made on the `/notifications` page (or by this dropdown).
  useEffect(
    () =>
      subscribeToChanges((change) => {
        void refreshCount();
        setRecent((list) => list && applyChange(list, change));
      }),
    [refreshCount],
  );

  // On wide screens the dropdown is a fixed flyout beside the sidebar (the sidebar scrolls,
  // so anything absolutely positioned inside it would be clipped). Anchor it to the bell.
  const place = useCallback(() => {
    const rect = button.current?.getBoundingClientRect();
    if (!rect) return;
    setPosition({
      "--flyout-left": `${rect.right + 8}px`,
      "--flyout-bottom": `${Math.max(8, window.innerHeight - rect.bottom)}px`,
    } as CSSProperties);
  }, []);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (e: PointerEvent) => {
      if (!wrapper.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setOpen(false);
      button.current?.focus();
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    window.addEventListener("resize", place);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("resize", place);
    };
  }, [open, place]);

  function toggle() {
    if (open) {
      setOpen(false);
      return;
    }
    place();
    setOpen(true);
    void loadRecent();
    void refreshCount();
  }

  /** Marks `n` read (it is being opened). A link row also closes the dropdown. */
  function follow(n: Notification, close: boolean) {
    if (close) setOpen(false);
    if (n.read_at) return;
    setRecent((list) => list && applyChange(list, { kind: "read", ids: [n.id] }));
    setCount((c) => Math.max(0, c - 1));
    void markNotificationRead(n.id).then((res) => {
      if (res.ok) announceChange({ kind: "read", ids: [n.id] });
      else void refreshCount();
    });
  }

  return (
    <div ref={wrapper} className={styles.bell}>
      <button
        ref={button}
        type="button"
        className={`${nav.link} ${styles.bellButton} ${open ? nav.active : ""}`}
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        aria-label={count > 0 ? `Notifications, ${count} unread` : "Notifications"}
        onClick={toggle}
      >
        <span className={nav.icon}>
          <BellIcon />
        </span>
        Notifications
        {count > 0 && (
          <span className={styles.badge} aria-hidden>
            {badgeText(count)}
          </span>
        )}
      </button>
      {open && (
        <div id={panelId} className={styles.panel} style={position} role="region" aria-label="Recent notifications">
          {failed ? (
            <p className={styles.panelNote}>
              Couldn&apos;t load your notifications.{" "}
              <button type="button" className={styles.textButton} onClick={loadRecent}>
                Try again
              </button>
            </p>
          ) : recent === null ? (
            <p className={styles.panelNote}>Loading...</p>
          ) : recent.length === 0 ? (
            <p className={styles.panelNote}>No notifications yet.</p>
          ) : (
            <ul className={styles.recent}>
              {recent.map((n) => (
                <li key={n.id}>
                  <RecentRow notification={n} onFollow={follow} />
                </li>
              ))}
            </ul>
          )}
          <Link className={styles.viewAll} href="/notifications" onClick={() => setOpen(false)}>
            View all
          </Link>
        </div>
      )}
    </div>
  );
}

function RecentRow({
  notification: n,
  onFollow,
}: {
  notification: Notification;
  onFollow: (n: Notification, close: boolean) => void;
}) {
  const href = notificationHref(n.url);
  const content = (
    <>
      <TypeIcon type={n.type} className={styles.typeIcon} />
      <span className={styles.recentText}>
        <span className={`${styles.recentTitle} ${n.read_at ? "" : styles.unreadTitle}`}>{n.title}</span>
        <span className={styles.meta}>
          <Ago iso={n.created_at} />
        </span>
      </span>
      {!n.read_at && (
        <span className={styles.dot}>
          <span className={styles.srOnly}>Unread</span>
        </span>
      )}
    </>
  );
  // A real link where there is somewhere to go, so the mobile overlay closes itself.
  return href ? (
    <Link className={styles.recentRow} href={href} onClick={() => onFollow(n, true)}>
      {content}
    </Link>
  ) : (
    <button type="button" className={styles.recentRow} onClick={() => onFollow(n, false)}>
      {content}
    </button>
  );
}
