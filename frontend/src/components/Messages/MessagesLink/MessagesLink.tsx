"use client";

import { useCallback, useEffect, useState } from "react";
import { NavLink } from "@/components/SideNav/NavLink";
import { MessagesIcon } from "@/components/SideNav/icons";
import { UNREAD_POLL_INTERVAL_MS } from "@/lib/messageEnums";
import { badgeText } from "@/lib/notificationEnums";
import { subscribeToMessagesChanges } from "../events";
import styles from "./MessagesLink.module.scss";

/**
 * The "Messages" link in the side navigation with the unread count, refreshed every minute
 * while the tab is visible and whenever a chat is read or written to (see `../events`).
 * `initialCount` comes from the server render, which counts as the page-load fetch.
 */
export function MessagesLink({ initialCount }: { initialCount: number }) {
  const [count, setCount] = useState(initialCount);

  const refreshCount = useCallback(async () => {
    try {
      const res = await fetch("/api/messages/unread-count", { cache: "no-store" });
      // Logged out in another tab, the session expired, or messaging was taken away.
      if (res.status === 401 || res.status === 403) setCount(0);
      else if (res.ok) setCount(((await res.json()) as { count: number }).count);
    } catch {
      // Offline or the server is restarting; the next poll tries again.
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
      timer = window.setInterval(refreshCount, UNREAD_POLL_INTERVAL_MS);
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

  useEffect(() => subscribeToMessagesChanges(() => void refreshCount()), [refreshCount]);

  return (
    <div className={styles.wrapper}>
      <NavLink
        href="/messages"
        icon={<MessagesIcon />}
        aria-label={count > 0 ? `Messages, ${count} unread` : undefined}
        badge={
          count > 0 ? (
            <span className={styles.badge} aria-hidden>
              {badgeText(count)}
            </span>
          ) : null
        }
      >
        Messages
      </NavLink>
    </div>
  );
}
