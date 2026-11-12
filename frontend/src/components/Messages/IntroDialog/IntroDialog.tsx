"use client";

import { useEffect, useRef, useState } from "react";
import styles from "./IntroDialog.module.scss";

/**
 * Confirmed in this tab. Not stored anywhere (docs/PRIVATE_MESSAGES.md): the popup shows on
 * every visit until the user has sent a message, but not again on each page of one visit.
 */
let confirmedThisVisit = false;

/** Whether the user confirmed the popup earlier in this tab. */
export function introConfirmed(): boolean {
  return confirmedThisVisit;
}

/**
 * Tells the user that messages aren't private. Until they have sent a message (`hasSent`),
 * a popup explains it and asks them to confirm before writing (`onConfirm`); closing it
 * without confirming leaves a line to reopen it. Afterwards, a grayed-out disclaimer.
 */
export function IntroDialog({ hasSent, onConfirm }: { hasSent: boolean; onConfirm?: () => void }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [confirmed, setConfirmed] = useState(() => hasSent || confirmedThisVisit);

  useEffect(() => {
    const d = dialog.current;
    if (!confirmed && d && !d.open) d.showModal();
    // Only on arrival; afterwards the user reopens it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (confirmed) {
    return (
      <p className={styles.disclaimer}>
        Messages aren&apos;t private: staff who manage users can read them, including deleted and disappeared ones.
      </p>
    );
  }

  function confirm() {
    confirmedThisVisit = true;
    dialog.current?.close();
    setConfirmed(true);
    onConfirm?.();
  }

  return (
    <>
      <p className={styles.locked} role="status">
        Before writing, read how private messages work.{" "}
        <button type="button" className={styles.textButton} onClick={() => dialog.current?.showModal()}>
          Read and confirm
        </button>
      </p>
      <dialog
        ref={dialog}
        className={styles.dialog}
        aria-labelledby="messages-intro-title"
        // Escape would skip the confirmation; "Not now" closes it instead.
        onCancel={(e) => e.preventDefault()}
      >
        <div className={styles.body}>
          <h2 id="messages-intro-title">Messages aren&apos;t private</h2>
          <p>
            Messages are between you and the other user, but staff who manage users can read every chat. That includes
            chats either of you deleted, and disappearing messages after they disappear for you.
          </p>
          <p>Don&apos;t send anything you wouldn&apos;t want staff to see, such as passwords or personal details.</p>
          <div className={styles.actions}>
            <button type="button" className={styles.secondary} onClick={() => dialog.current?.close()}>
              Not now
            </button>
            <button type="button" className={styles.primary} onClick={confirm} autoFocus>
              I understand
            </button>
          </div>
        </div>
      </dialog>
    </>
  );
}
