"use client";

import { useId, useState, useTransition, type FormEvent, type KeyboardEvent } from "react";
import { AttachmentPicker, useAttachments } from "@/components/Upload";
import { MAX_PM_ATTACHMENTS } from "@/lib/files";
import { MAX_CONTENT } from "@/lib/messageEnums";
import type { PrivateMessage } from "@/lib/types";
import { sendMessage } from "../actions";
import styles from "./Composer.module.scss";

type Props = {
  userId: number;
  username: string;
  /** Called with the message once it's sent. */
  onSent: (message: PrivateMessage) => void;
  /** Until the user confirms the popup about privacy. */
  disabled?: boolean;
};

/** The message box at the bottom of a chat: Markdown, up to 3 files and the "disappearing" toggle. */
export function Composer({ userId, username, onSent, disabled = false }: Props) {
  const [content, setContent] = useState("");
  const [temporary, setTemporary] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const attachments = useAttachments(MAX_PM_ATTACHMENTS);
  const ids = useId();

  const canSend = !disabled && !pending && !attachments.busy && content.trim() !== "";

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSend) return;
    startTransition(async () => {
      const res = await sendMessage(userId, content.trim(), attachments.ids, temporary);
      if (!res.ok) {
        setError(res.message);
        return;
      }
      setError(null);
      setContent("");
      setTemporary(false);
      attachments.clear();
      onSent(res.data);
    });
  }

  /** Ctrl+Enter (or Cmd+Enter) sends; Enter alone starts a new line. */
  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
    }
  }

  return (
    <form className={styles.composer} onSubmit={onSubmit} aria-label={`Message ${username}`}>
      <textarea
        id={`${ids}-content`}
        className={styles.textarea}
        value={content}
        onChange={(e) => setContent(e.target.value)}
        onKeyDown={onKeyDown}
        maxLength={MAX_CONTENT}
        rows={3}
        placeholder={disabled ? "Confirm the notice below to start writing." : `Message ${username}...`}
        aria-label="Message"
        aria-describedby={`${ids}-hint`}
        disabled={disabled}
        required
      />
      <div className={styles.meta}>
        <span id={`${ids}-hint`}>Markdown supported · Ctrl+Enter to send</span>
        <span className={content.length > MAX_CONTENT * 0.9 ? styles.countNear : undefined}>
          {content.length.toLocaleString("en")} / {MAX_CONTENT.toLocaleString("en")}
        </span>
      </div>
      <AttachmentPicker {...attachments.picker} disabled={disabled || pending} />
      <div className={styles.actions}>
        <button type="submit" className={styles.send} disabled={!canSend}>
          {pending ? "Sending..." : "Send"}
        </button>
        <label className={styles.checkbox}>
          <input
            type="checkbox"
            checked={temporary}
            onChange={(e) => setTemporary(e.target.checked)}
            disabled={disabled || pending}
          />
          Disappearing message (24h)
        </label>
        {error && (
          <p className={styles.error} role="alert">
            {error}
          </p>
        )}
      </div>
    </form>
  );
}
