"use client";

import { useState, useTransition, type FormEvent } from "react";
import { createThread, replyToThread } from "./actions";
import styles from "./Forum.module.scss";

/** Limits enforced by the backend (src/routes/forum.rs). */
const MAX_TITLE = 128;
const MAX_CONTENT = 20_000;

/** "New thread" button that opens a title + content form. Opens the thread once posted. */
export function NewThreadForm({ categoryId }: { categoryId: number }) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  if (!open) {
    return (
      <button type="button" className={styles.primary} onClick={() => setOpen(true)}>
        New thread
      </button>
    );
  }

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startTransition(async () => {
      // Redirects to the new thread on success.
      const res = await createThread(categoryId, title.trim(), content.trim());
      if (!res.ok) setError(res.message);
    });
  }

  return (
    <form className={`${styles.composer} ${styles.composerWide}`} onSubmit={onSubmit}>
      <label className={styles.field}>
        <span className={styles.label}>Title</span>
        <input
          className={styles.input}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          maxLength={MAX_TITLE}
          required
          autoFocus
        />
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Post (Markdown supported)</span>
        <textarea
          className={`${styles.input} ${styles.textarea}`}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          maxLength={MAX_CONTENT}
          rows={8}
          required
        />
      </label>
      <div className={styles.composerActions}>
        <button type="submit" className={styles.primary} disabled={pending || !title.trim() || !content.trim()}>
          {pending ? "Posting..." : "Post thread"}
        </button>
        <button type="button" className={styles.secondary} onClick={() => setOpen(false)} disabled={pending}>
          Cancel
        </button>
        {error && (
          <p className={styles.error} role="status">
            {error}
          </p>
        )}
      </div>
    </form>
  );
}

/** Reply box at the bottom of a thread. */
export function ReplyForm({ threadId }: { threadId: number }) {
  const [content, setContent] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startTransition(async () => {
      const res = await replyToThread(threadId, content.trim());
      if (res.ok) {
        setContent("");
        setError(null);
      } else {
        setError(res.message);
      }
    });
  }

  return (
    <form className={styles.composer} onSubmit={onSubmit} aria-label="Reply">
      <textarea
        className={`${styles.input} ${styles.textarea}`}
        value={content}
        onChange={(e) => setContent(e.target.value)}
        maxLength={MAX_CONTENT}
        rows={5}
        placeholder="Write a reply... (Markdown supported)"
        aria-label="Reply"
        required
      />
      <div className={styles.composerActions}>
        <button type="submit" className={styles.primary} disabled={pending || !content.trim()}>
          {pending ? "Posting..." : "Reply"}
        </button>
        {error && (
          <p className={styles.error} role="status">
            {error}
          </p>
        )}
      </div>
    </form>
  );
}
