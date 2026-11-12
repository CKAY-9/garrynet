"use client";

import { useRouter } from "next/navigation";
import { useRef, useState } from "react";
import { UserPicker } from "@/components/UserPicker";
import { chatHref } from "@/lib/messageEnums";
import type { UserRef } from "@/lib/types";
import styles from "./NewMessage.module.scss";

/** The "New message" button on `/messages`: find a user by name or id, then open the chat with them. */
export function NewMessage({ viewerId }: { viewerId: number }) {
  const dialog = useRef<HTMLDialogElement>(null);
  const router = useRouter();
  // Remounted on every open so the search starts empty.
  const [opened, setOpened] = useState(0);

  function open() {
    setOpened((n) => n + 1);
    dialog.current?.showModal();
  }

  function onSelect(user: UserRef) {
    dialog.current?.close();
    router.push(chatHref(user.id));
  }

  return (
    <>
      <button type="button" className={styles.trigger} onClick={open}>
        New message
      </button>
      <dialog ref={dialog} className={styles.dialog} aria-labelledby="new-message-title">
        <div className={styles.body}>
          <h2 id="new-message-title">New message</h2>
          {opened > 0 && (
            <UserPicker
              key={opened}
              mode="single"
              onSelect={onSelect}
              exclude={[viewerId]}
              hint="Pick who to write to. A chat you already have with them opens instead."
              autoFocus
            />
          )}
          <div className={styles.actions}>
            <button type="button" className={styles.cancel} onClick={() => dialog.current?.close()}>
              Cancel
            </button>
          </div>
        </div>
      </dialog>
    </>
  );
}
