"use client";

import Link from "next/link";
import { useCallback, useEffect, useState } from "react";
import { Avatar } from "@/components/Avatar";
import { Ago } from "@/components/Notifications/parts";
import { PAGE_SIZE, pageParams, THREAD_POLL_INTERVAL_MS } from "@/lib/messageEnums";
import type { MessageThread, PrivateMessage } from "@/lib/types";
import { markChatRead } from "../actions";
import { Composer } from "../Composer";
import { DeleteChat } from "../DeleteChat";
import { announceMessagesChange } from "../events";
import { IntroDialog, introConfirmed } from "../IntroDialog";
import { MessageRow, OlderStatus } from "./parts";
import styles from "./Thread.module.scss";
import { useMessagePages } from "./useMessagePages";

type Props = {
  viewerId: number;
  /** The most recent page, from the server render. */
  initial: MessageThread;
  /** Whether the viewer has ever sent a message; until then they confirm the privacy popup first. */
  hasSent: boolean;
};

/**
 * A chat with one user (`/messages/{user_id}`): a header with the delete button, the messages
 * oldest first, and the composer. Older messages load when scrolling up; newer ones are
 * fetched every 15 seconds while the tab is visible. Opening it marks the chat read.
 */
export function Thread({ viewerId, initial, hasSent: initialHasSent }: Props) {
  const other = initial.user;
  const {
    messages,
    merge,
    stickToBottom,
    cursor,
    loadingOlder,
    olderFailed,
    loadOlder,
    scroller,
    content,
    sentinel,
    onScroll,
  } = useMessagePages(initial, `/api/messages/${other.id}`);
  const [hasSent, setHasSent] = useState(initialHasSent);
  const [confirmed, setConfirmed] = useState(() => initialHasSent || introConfirmed());

  const markRead = useCallback(() => {
    void markChatRead(other.id).then((res) => {
      if (res.ok && res.data > 0) announceMessagesChange();
    });
  }, [other.id]);

  // Opening the chat reads it. Reading marks everything they sent, so if the newest page has
  // nothing unread, neither do older ones.
  useEffect(() => {
    if (initial.items.some((m) => m.author_id === other.id && !m.read_at)) markRead();
  }, [initial.items, other.id, markRead]);

  // Fetch newer messages while the tab is visible. Every backend request marks the user
  // online, so a hidden tab must not keep polling; coming back fetches at once.
  useEffect(() => {
    let timer: number | undefined;
    const poll = async () => {
      try {
        const res = await fetch(`/api/messages/${other.id}?${pageParams(null, PAGE_SIZE)}`, { cache: "no-store" });
        if (!res.ok) return;
        const page = (await res.json()) as MessageThread;
        merge(page.items);
        if (page.items.some((m) => m.author_id === other.id && !m.read_at)) markRead();
      } catch {
        // Offline or the server is restarting; the next poll tries again.
      }
    };
    const stop = () => {
      window.clearInterval(timer);
      timer = undefined;
    };
    const start = () => {
      stop();
      timer = window.setInterval(poll, THREAD_POLL_INTERVAL_MS);
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void poll();
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
  }, [other.id, markRead, merge]);

  function onSent(message: PrivateMessage) {
    stickToBottom();
    merge([message]);
    setHasSent(true);
    announceMessagesChange();
  }

  return (
    <section className={styles.chat} aria-label={`Chat with ${other.username}`}>
      <header className={styles.header}>
        <Link href="/messages" className={styles.back} aria-label="All messages" title="All messages">
          ←
        </Link>
        <Link href={`/users/${other.id}`} className={styles.who}>
          <Avatar name={other.username} url={other.avatar_url || null} size={36} shape="circle" />
          <span className={styles.name}>{other.username}</span>
        </Link>
        <DeleteChat userId={other.id} username={other.username} />
      </header>

      <div ref={scroller} className={styles.scroller} onScroll={onScroll}>
        <OlderStatus
          ref={sentinel}
          hasMore={Boolean(cursor)}
          loading={loadingOlder}
          failed={olderFailed}
          onRetry={() => void loadOlder()}
          start={`This is the start of your chat with ${other.username}.`}
          any={messages.length > 0}
        />
        <ol ref={content} className={styles.messages} aria-label="Messages">
          {messages.map((m) => (
            <MessageRow key={m.id} message={m} mine={m.author_id === viewerId}>
              <Ago iso={m.created_at} />
              {m.temporary && (
                <span className={styles.temporary} title="Hidden from both of you 24 hours after it was sent">
                  Disappears after 24h
                </span>
              )}
            </MessageRow>
          ))}
        </ol>
        {messages.length === 0 && !cursor && (
          <p className={styles.empty}>No messages yet. Say hello to {other.username}!</p>
        )}
      </div>

      <Composer userId={other.id} username={other.username} onSent={onSent} disabled={!confirmed} />
      <div className={styles.notice}>
        <IntroDialog hasSent={hasSent} onConfirm={() => setConfirmed(true)} />
      </div>
    </section>
  );
}
