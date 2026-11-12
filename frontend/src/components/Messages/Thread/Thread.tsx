"use client";

import Link from "next/link";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Avatar } from "@/components/Avatar";
import { Markdown } from "@/components/Markdown";
import { Ago } from "@/components/Notifications/parts";
import { Attachments } from "@/components/Upload";
import { PAGE_SIZE, pageParams, THREAD_POLL_INTERVAL_MS } from "@/lib/messageEnums";
import type { MessageThread, PrivateMessage } from "@/lib/types";
import { markChatRead } from "../actions";
import { Composer } from "../Composer";
import { DeleteChat } from "../DeleteChat";
import { announceMessagesChange } from "../events";
import { IntroDialog, introConfirmed } from "../IntroDialog";
import styles from "./Thread.module.scss";

type Props = {
  viewerId: number;
  /** The most recent page, from the server render. */
  initial: MessageThread;
  /** Whether the viewer has ever sent a message; until then they confirm the privacy popup first. */
  hasSent: boolean;
};

/** How close to the bottom (px) still counts as reading the newest messages. */
const BOTTOM_SLACK = 64;

/**
 * A chat with one user (`/messages/{user_id}`): a header with the delete button, the messages
 * oldest first, and the composer. Older messages load when scrolling up; newer ones are
 * fetched every 15 seconds while the tab is visible. Opening it marks the chat read.
 */
export function Thread({ viewerId, initial, hasSent: initialHasSent }: Props) {
  const other = initial.user;
  const [messages, setMessages] = useState(() => mergeMessages([], initial.items));
  const [cursor, setCursor] = useState(initial.next_cursor);
  const [loadingOlder, setLoadingOlder] = useState(false);
  const [olderFailed, setOlderFailed] = useState(false);
  const [hasSent, setHasSent] = useState(initialHasSent);
  const [confirmed, setConfirmed] = useState(() => initialHasSent || introConfirmed());

  const scroller = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLOListElement>(null);
  const sentinel = useRef<HTMLDivElement>(null);
  /** Whether the view ends at the newest message, so it should stay there as content grows. */
  const atBottom = useRef(true);
  /** Set just before older messages are prepended: the scroll position to keep the view on. */
  const anchor = useRef<{ height: number; top: number } | null>(null);
  const loadingRef = useRef(false);

  // Keep the view still when older messages are prepended, or at the bottom when it was there.
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el) return;
    if (anchor.current) {
      el.scrollTop = anchor.current.top + (el.scrollHeight - anchor.current.height);
      anchor.current = null;
    } else if (atBottom.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages]);

  // Images and other late layout changes: stay at the bottom if the view was there.
  useEffect(() => {
    const el = scroller.current;
    const list = content.current;
    if (!el || !list) return;
    const observer = new ResizeObserver(() => {
      if (atBottom.current && !anchor.current) el.scrollTop = el.scrollHeight;
    });
    observer.observe(list);
    return () => observer.disconnect();
  }, []);

  function onScroll() {
    const el = scroller.current;
    if (el) atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < BOTTOM_SLACK;
  }

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

  const loadOlder = useCallback(async () => {
    if (!cursor || loadingRef.current) return;
    loadingRef.current = true;
    setLoadingOlder(true);
    setOlderFailed(false);
    try {
      const res = await fetch(`/api/messages/${other.id}?${pageParams(cursor, PAGE_SIZE)}`, { cache: "no-store" });
      if (!res.ok) throw new Error(`status ${res.status}`);
      const page = (await res.json()) as MessageThread;
      const el = scroller.current;
      if (el) anchor.current = { height: el.scrollHeight, top: el.scrollTop };
      setMessages((list) => mergeMessages(list, page.items));
      setCursor(page.next_cursor);
    } catch {
      setOlderFailed(true);
    } finally {
      loadingRef.current = false;
      setLoadingOlder(false);
    }
  }, [cursor, other.id]);

  // Load older messages when the top comes into view. Re-observing after each page fires
  // again at once if the top is still in view, so a short chat fills the box.
  useEffect(() => {
    const root = scroller.current;
    const target = sentinel.current;
    if (!root || !target || !cursor || loadingOlder || olderFailed) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) void loadOlder();
      },
      { root, rootMargin: "200px 0px 0px 0px" },
    );
    observer.observe(target);
    return () => observer.disconnect();
  }, [cursor, loadingOlder, olderFailed, loadOlder]);

  // Fetch newer messages while the tab is visible. Every backend request marks the user
  // online, so a hidden tab must not keep polling; coming back fetches at once.
  useEffect(() => {
    let timer: number | undefined;
    const poll = async () => {
      try {
        const res = await fetch(`/api/messages/${other.id}?${pageParams(null, PAGE_SIZE)}`, { cache: "no-store" });
        if (!res.ok) return;
        const page = (await res.json()) as MessageThread;
        setMessages((list) => mergeMessages(list, page.items));
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
  }, [other.id, markRead]);

  function onSent(message: PrivateMessage) {
    atBottom.current = true;
    setMessages((list) => mergeMessages(list, [message]));
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
        <div ref={sentinel} className={styles.older} role="status">
          {olderFailed ? (
            <>
              Couldn&apos;t load older messages.{" "}
              <button type="button" className={styles.textButton} onClick={() => void loadOlder()}>
                Try again
              </button>
            </>
          ) : loadingOlder ? (
            "Loading older messages..."
          ) : !cursor && messages.length > 0 ? (
            `This is the start of your chat with ${other.username}.`
          ) : null}
        </div>
        <ol ref={content} className={styles.messages} aria-label="Messages">
          {messages.map((m) => (
            <MessageRow key={m.id} message={m} mine={m.author_id === viewerId} />
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

function MessageRow({ message: m, mine }: { message: PrivateMessage; mine: boolean }) {
  return (
    <li className={`${styles.message} ${mine ? styles.mine : styles.theirs}`}>
      <div className={styles.bubble}>
        <Markdown className={styles.content}>{m.content}</Markdown>
        <Attachments files={m.attachments} />
      </div>
      <span className={styles.meta}>
        <Ago iso={m.created_at} />
        {m.temporary && (
          <span className={styles.temporary} title="Hidden from both of you 24 hours after it was sent">
            Disappears after 24h
          </span>
        )}
      </span>
    </li>
  );
}

/**
 * `list` with `incoming` merged in by id (the incoming copy wins, e.g. once a message is read),
 * oldest first. Returns `list` itself when nothing changed, so a quiet poll doesn't re-render.
 */
function mergeMessages(list: PrivateMessage[], incoming: PrivateMessage[]): PrivateMessage[] {
  const byId = new Map(list.map((m) => [m.id, m]));
  let changed = false;
  for (const m of incoming) {
    const current = byId.get(m.id);
    if (current && current.read_at === m.read_at) continue;
    byId.set(m.id, m);
    changed = true;
  }
  if (!changed) return list;
  return [...byId.values()].sort((a, b) => Date.parse(a.created_at) - Date.parse(b.created_at) || a.id - b.id);
}
