import type { ReactNode, Ref } from "react";
import { Markdown } from "@/components/Markdown";
import { Attachments } from "@/components/Upload";
import type { PrivateMessage } from "@/lib/types";
import styles from "./Thread.module.scss";

type Props = {
  message: PrivateMessage;
  /** Right-aligned, in the accent colour; otherwise left-aligned. */
  mine: boolean;
  className?: string;
  /** The line under the bubble: when it was sent and any labels. */
  children: ReactNode;
};

/** One message in a chat: a bubble with its Markdown and attachments, and a meta line under it. */
export function MessageRow({ message: m, mine, className = "", children }: Props) {
  return (
    <li className={`${styles.message} ${mine ? styles.mine : styles.theirs} ${className}`}>
      <div className={styles.bubble}>
        <Markdown className={styles.content}>{m.content}</Markdown>
        <Attachments files={m.attachments} />
      </div>
      <span className={styles.meta}>{children}</span>
    </li>
  );
}

type OlderStatusProps = {
  /** The `sentinel` from `useMessagePages`. */
  ref: Ref<HTMLDivElement>;
  hasMore: boolean;
  loading: boolean;
  failed: boolean;
  onRetry: () => void;
  /** Shown above the first message once every page is loaded. */
  start: string;
  /** Whether there are any messages to be the start of. */
  any: boolean;
};

/** Above the messages: loading older ones, a retry after a failure, or the start of the chat. */
export function OlderStatus({ ref, hasMore, loading, failed, onRetry, start, any }: OlderStatusProps) {
  return (
    <div ref={ref} className={styles.older} role="status">
      {failed ? (
        <>
          Couldn&apos;t load older messages.{" "}
          <button type="button" className={styles.textButton} onClick={onRetry}>
            Try again
          </button>
        </>
      ) : loading ? (
        "Loading older messages..."
      ) : !hasMore && any ? (
        start
      ) : null}
    </div>
  );
}
