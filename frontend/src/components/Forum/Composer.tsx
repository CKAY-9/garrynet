"use client";

import { useState, useTransition, type FormEvent } from "react";
import { AttachmentPicker, useAttachments } from "@/components/Upload";
import { MAX_POST_ATTACHMENTS } from "@/lib/files";
import { createThread, replyToThread } from "./actions";
import styles from "./Forum.module.scss";

/** Limits enforced by the backend (src/routes/forum.rs). */
const MAX_TITLE = 128;
const MAX_CONTENT = 20_000;

/** A thread to copy into a new thread's form; see "Use as template". */
export type ThreadTemplate = { title: string; content: string };

/**
 * "New thread" button that opens a title + content form. Opens the thread once posted.
 * Starts open and filled in when given a `template`. Moderators can also start it locked.
 */
export function NewThreadForm({
  categoryId,
  moderator,
  template,
}: {
  categoryId: number;
  moderator: boolean;
  template?: ThreadTemplate;
}) {
  const [open, setOpen] = useState(template !== undefined);
  const [title, setTitle] = useState(template?.title ?? "");
  const [content, setContent] = useState(template?.content ?? "");
  const [allowTemplate, setAllowTemplate] = useState(false);
  const [locked, setLocked] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const attachments = useAttachments(MAX_POST_ATTACHMENTS);

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
      const res = await createThread(categoryId, {
        title: title.trim(),
        content: content.trim(),
        allow_template: allowTemplate,
        locked: moderator && locked,
        attachments: attachments.ids,
      });
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
      <AttachmentPicker {...attachments.picker} disabled={pending} />
      <div className={styles.options}>
        <label className={styles.checkbox}>
          <input type="checkbox" checked={allowTemplate} onChange={(e) => setAllowTemplate(e.target.checked)} />
          <span>Allow others to use this thread as a template</span>
        </label>
        {moderator && (
          <label className={styles.checkbox}>
            <input type="checkbox" checked={locked} onChange={(e) => setLocked(e.target.checked)} />
            <span>Lock thread (no replies)</span>
          </label>
        )}
      </div>
      <div className={styles.composerActions}>
        <button type="submit" className={styles.primary} disabled={pending || attachments.busy || !title.trim() || !content.trim()}>
          {pending ? "Posting..." : "Post thread"}
        </button>
        <button
          type="button"
          className={styles.secondary}
          onClick={() => {
            // The attached files won't be posted; delete them.
            attachments.clear({ discard: true });
            setOpen(false);
          }}
          disabled={pending || attachments.busy}
        >
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
  const attachments = useAttachments(MAX_POST_ATTACHMENTS);

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startTransition(async () => {
      const res = await replyToThread(threadId, content.trim(), attachments.ids);
      if (res.ok) {
        setContent("");
        attachments.clear();
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
      <AttachmentPicker {...attachments.picker} disabled={pending} />
      <div className={styles.composerActions}>
        <button type="submit" className={styles.primary} disabled={pending || attachments.busy || !content.trim()}>
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
