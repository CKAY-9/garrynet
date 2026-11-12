"use client";

import Link from "next/link";
import { useState } from "react";
import { Avatar } from "@/components/Avatar";
import { Ago } from "@/components/Notifications/parts";
import { PAGE_SIZE, pageParams } from "@/lib/messageEnums";
import { badgeText } from "@/lib/notificationEnums";
import type { Chat, ChatPage } from "@/lib/types";
import styles from "./ChatList.module.scss";

type Props = {
  /** The first page, from the server render. */
  initial: Pick<ChatPage, "items" | "next_cursor">;
  /** Whose chats these are. */
  viewerId: number;
  /** Route handler serving later pages. */
  endpoint?: string;
  /** Each chat links to `{chatBase}/{other user's id}`. */
  chatBase?: string;
  /** Marks the last message when `viewerId` wrote it. */
  selfLabel?: string;
  /** Shown when there are no chats. */
  empty?: string;
};

/**
 * Chats, most recent first: the first page from the server, then "Load more". Each shows the
 * other user, the last message and how many are unread. The defaults are the current user's
 * own list on `/messages`; staff reuse it for anyone's on `/admin/messages/{user_id}`.
 */
export function ChatList({
  initial,
  viewerId,
  endpoint = "/api/messages",
  chatBase = "/messages",
  selfLabel = "You",
  empty = "No chats yet. Use New message to start one.",
}: Props) {
  const [chats, setChats] = useState(initial.items);
  const [cursor, setCursor] = useState(initial.next_cursor);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function loadMore() {
    if (!cursor) return;
    setLoading(true);
    setError(null);
    let failure = "Couldn't load more chats. Try again.";
    try {
      const res = await fetch(`${endpoint}?${pageParams(cursor, PAGE_SIZE)}`, { cache: "no-store" });
      if (res.ok) {
        const page = (await res.json()) as Pick<ChatPage, "items" | "next_cursor">;
        setChats((list) => mergeChats(list, page.items));
        setCursor(page.next_cursor);
        return;
      }
      if (res.status === 401) failure = "Your session has expired. Log in again to see more chats.";
    } catch {
      // Network failure; keep the generic message.
    } finally {
      setLoading(false);
    }
    setError(failure);
  }

  return (
    <section className={styles.list} aria-label="Chats">
      <ul className={styles.rows}>
        {chats.map((chat) => (
          <li key={chat.user.id}>
            <ChatRow chat={chat} viewerId={viewerId} href={`${chatBase}/${chat.user.id}`} selfLabel={selfLabel} />
          </li>
        ))}
        {chats.length === 0 && !cursor && (
          <li className={styles.empty}>{empty}</li>
        )}
      </ul>
      {error && (
        <p className={styles.error} role="alert">
          {error}
        </p>
      )}
      {cursor && (
        <button type="button" className={styles.loadMore} onClick={loadMore} disabled={loading}>
          {loading ? "Loading..." : "Load more"}
        </button>
      )}
    </section>
  );
}

function ChatRow({ chat, viewerId, href, selfLabel }: { chat: Chat; viewerId: number; href: string; selfLabel: string }) {
  const unread = chat.unread_count > 0;
  return (
    <Link href={href} className={`${styles.row} ${unread ? styles.unread : ""}`}>
      <Avatar name={chat.user.username} url={chat.user.avatar_url || null} size={40} shape="circle" />
      <span className={styles.main}>
        <span className={styles.top}>
          <span className={styles.name}>{chat.user.username}</span>
          <span className={styles.when}>
            <Ago iso={chat.last_at} />
          </span>
        </span>
        <span className={styles.preview}>
          {chat.last_author_id === viewerId && <span className={styles.you}>{selfLabel}: </span>}
          {chat.preview}
        </span>
      </span>
      {unread && (
        <span className={styles.count} aria-label={`${chat.unread_count} unread`}>
          {badgeText(chat.unread_count)}
        </span>
      )}
    </Link>
  );
}

/** `list` with `page` appended. A chat with a new message moves up, so it may come back in a later page: keep the first copy. */
function mergeChats(list: Chat[], page: Chat[]): Chat[] {
  const seen = new Set(list.map((c) => c.user.id));
  return [...list, ...page.filter((c) => !seen.has(c.user.id))];
}
