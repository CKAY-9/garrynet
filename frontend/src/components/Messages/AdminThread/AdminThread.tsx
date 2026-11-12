"use client";

import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { Ago } from "@/components/Notifications/parts";
import type { AdminMessage, AdminMessageThread, UserRef } from "@/lib/types";
import { MessageRow, OlderStatus } from "../Thread/parts";
import thread from "../Thread/Thread.module.scss";
import { useMessagePages } from "../Thread/useMessagePages";
import styles from "./AdminThread.module.scss";

/**
 * Staff's read-only view of the chat between two users (`/admin/messages/{user_id}/{other_id}`):
 * every message, including ones either user deleted and temporary ones past 24 hours, each
 * marked. `user`'s messages are on the right. Older messages load when scrolling up. There is
 * no composer, no polling and no read receipt: viewing never marks anything read.
 */
export function AdminThread({ initial }: { initial: AdminMessageThread }) {
  const { user, other } = initial;
  const { messages, cursor, loadingOlder, olderFailed, loadOlder, scroller, content, sentinel, onScroll } =
    useMessagePages(initial, `/api/admin/messages/${user.id}/${other.id}`);
  const name = (id: number) => (id === user.id ? user.username : id === other.id ? other.username : `#${id}`);

  return (
    <section className={styles.chat} aria-label={`Chat between ${user.username} and ${other.username}`}>
      <header className={styles.header}>
        <Person user={user} />
        <span className={styles.and} aria-hidden>
          ⇄
        </span>
        <Person user={other} />
      </header>

      <div ref={scroller} className={thread.scroller} onScroll={onScroll}>
        <OlderStatus
          ref={sentinel}
          hasMore={Boolean(cursor)}
          loading={loadingOlder}
          failed={olderFailed}
          onRetry={() => void loadOlder()}
          start={`This is the start of the chat between ${user.username} and ${other.username}.`}
          any={messages.length > 0}
        />
        <ol ref={content} className={thread.messages} aria-label="Messages">
          {messages.map((m) => (
            <MessageRow
              key={m.id}
              message={m}
              mine={m.author_id === user.id}
              className={m.deleted || m.expired ? styles.hidden : ""}
            >
              <span className={styles.author}>{name(m.author_id)}</span>
              <Ago iso={m.created_at} />
              <Labels message={m} name={name} />
            </MessageRow>
          ))}
        </ol>
        {messages.length === 0 && !cursor && (
          <p className={thread.empty}>
            {user.username} and {other.username} have no messages.
          </p>
        )}
      </div>
    </section>
  );
}

function Person({ user }: { user: UserRef }) {
  return (
    <Link href={`/users/${user.id}`} className={thread.who}>
      <Avatar name={user.username} url={user.avatar_url || null} size={32} shape="circle" />
      <span className={thread.name}>{user.username}</span>
    </Link>
  );
}

/** Whether either user deleted the message, whether it expired, and whether it was temporary or read. */
function Labels({ message: m, name }: { message: AdminMessage; name: (id: number) => string }) {
  const deletedBy =
    m.deleted_by_author && m.deleted_by_recipient
      ? "both users"
      : m.deleted_by_author
        ? name(m.author_id)
        : m.deleted_by_recipient
          ? name(m.recipient_id)
          : null;
  return (
    <>
      {m.deleted && (
        <span className={`${styles.badge} ${styles.deleted}`} title="Deleted from their chat; still kept for staff">
          Deleted{deletedBy ? ` by ${deletedBy}` : ""}
        </span>
      )}
      {m.expired ? (
        <span
          className={`${styles.badge} ${styles.expired}`}
          title="A temporary message over 24 hours old, so hidden from both users"
        >
          Expired
        </span>
      ) : (
        m.temporary && (
          <span className={thread.temporary} title="Hidden from both users 24 hours after it was sent">
            Temporary
          </span>
        )
      )}
      {!m.read_at && <span title={`${name(m.recipient_id)} hasn't opened it`}>Unread</span>}
    </>
  );
}
