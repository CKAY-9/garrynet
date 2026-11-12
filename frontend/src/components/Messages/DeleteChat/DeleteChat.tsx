"use client";

import { useRef, useState, useTransition } from "react";
import { TrashIcon } from "@/components/SideNav/icons";
import { deleteChat } from "../actions";
import styles from "./DeleteChat.module.scss";

/**
 * The trash can in a chat's header. Asks for confirmation in a dialog, then deletes the chat
 * for the current user only and opens the chat list.
 */
export function DeleteChat({ userId, username }: { userId: number; username: string }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function open() {
    setError(null);
    dialog.current?.showModal();
  }

  function confirm() {
    startTransition(async () => {
      // Redirects to /messages on success.
      const res = await deleteChat(userId);
      if (!res.ok) setError(res.message);
    });
  }

  return (
    <>
      <button type="button" className={styles.trigger} onClick={open} aria-label="Delete chat" title="Delete chat">
        <TrashIcon />
      </button>
      <dialog ref={dialog} className={styles.dialog} aria-labelledby={`delete-chat-${userId}`}>
        <div className={styles.body}>
          <h2 id={`delete-chat-${userId}`}>Delete your chat with {username}?</h2>
          <p>
            The messages are hidden from you only. {username} still sees the chat and isn&apos;t told. If either of you
            writes again, a new chat starts.
          </p>
          {error && (
            <p className={styles.error} role="alert">
              {error}
            </p>
          )}
          <div className={styles.actions}>
            <button type="button" className={styles.cancel} onClick={() => dialog.current?.close()} disabled={pending}>
              Cancel
            </button>
            <button type="button" className={styles.confirm} onClick={confirm} disabled={pending}>
              {pending ? "Deleting..." : "Delete chat"}
            </button>
          </div>
        </div>
      </dialog>
    </>
  );
}
