"use client";

import { useState, useTransition, type FormEvent, type ReactNode } from "react";
import type { ForumThread } from "@/lib/types";
import type { ActionResult } from "@/lib/actionResult";
import { deletePost, deleteThread, editPost, updateThread, type ThreadChanges } from "./actions";
import styles from "./Forum.module.scss";

/** Limits enforced by the backend (src/routes/forum.rs). */
const MAX_TITLE = 128;
const MAX_CONTENT = 20_000;

/** Runs a server action, keeping its error message for display. */
function useAction() {
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  function run<T>(action: () => Promise<ActionResult<T>>, onSuccess?: () => void) {
    startTransition(async () => {
      const res = await action();
      if (res.ok) {
        setError(null);
        onSuccess?.();
      } else {
        setError(res.message);
      }
    });
  }
  return { error, pending, run };
}

function ErrorText({ error }: { error: string | null }) {
  if (!error) return null;
  return (
    <p className={styles.error} role="status">
      {error}
    </p>
  );
}

/** A delete button that asks for confirmation inline before calling `onConfirm`. */
function DeleteButton({
  label,
  question,
  pending,
  onConfirm,
}: {
  label: string;
  question: string;
  pending: boolean;
  onConfirm: () => void;
}) {
  const [confirming, setConfirming] = useState(false);
  if (!confirming) {
    return (
      <button type="button" className={styles.tool} onClick={() => setConfirming(true)} disabled={pending}>
        {label}
      </button>
    );
  }
  return (
    <span className={styles.confirm} role="group" aria-label={question}>
      <span>{question}</span>
      <button type="button" className={`${styles.tool} ${styles.dangerTool}`} onClick={onConfirm} disabled={pending}>
        {pending ? "Deleting..." : "Delete"}
      </button>
      <button type="button" className={styles.tool} onClick={() => setConfirming(false)} disabled={pending}>
        Cancel
      </button>
    </span>
  );
}

/**
 * Controls under the thread title. Authors (while the thread is unlocked) and moderators
 * can rename it, toggle templating and delete it; only moderators can lock and sticky it.
 */
export function ThreadTools({
  thread,
  canManage,
  moderator,
}: {
  thread: ForumThread;
  canManage: boolean;
  moderator: boolean;
}) {
  const [renaming, setRenaming] = useState(false);
  const [title, setTitle] = useState(thread.title);
  const { error, pending, run } = useAction();

  if (!canManage) return null;

  const change = (changes: ThreadChanges, onSuccess?: () => void) => run(() => updateThread(thread.id, changes), onSuccess);

  function onRename(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    change({ title: title.trim() }, () => setRenaming(false));
  }

  if (renaming) {
    return (
      <form className={styles.composer} onSubmit={onRename} aria-label="Rename thread">
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
        <div className={styles.composerActions}>
          <button type="submit" className={styles.primary} disabled={pending || !title.trim()}>
            {pending ? "Saving..." : "Save title"}
          </button>
          <button
            type="button"
            className={styles.secondary}
            onClick={() => {
              setTitle(thread.title);
              setRenaming(false);
            }}
            disabled={pending}
          >
            Cancel
          </button>
          <ErrorText error={error} />
        </div>
      </form>
    );
  }

  return (
    <div className={styles.tools} aria-label="Thread tools">
      <button type="button" className={styles.tool} onClick={() => setRenaming(true)} disabled={pending}>
        Rename
      </button>
      <button
        type="button"
        className={styles.tool}
        onClick={() => change({ allow_template: !thread.allow_template })}
        disabled={pending}
      >
        {thread.allow_template ? "Disable template" : "Allow as template"}
      </button>
      {moderator && (
        <>
          <button type="button" className={styles.tool} onClick={() => change({ locked: !thread.locked })} disabled={pending}>
            {thread.locked ? "Unlock" : "Lock"}
          </button>
          <button type="button" className={styles.tool} onClick={() => change({ sticky: !thread.sticky })} disabled={pending}>
            {thread.sticky ? "Unsticky" : "Sticky"}
          </button>
        </>
      )}
      <DeleteButton
        label="Delete thread"
        question="Delete this thread and all its posts?"
        pending={pending}
        onConfirm={() => run(() => deleteThread(thread.id, thread.category_id))}
      />
      <ErrorText error={error} />
    </div>
  );
}

/**
 * A post's content (`children`, already rendered) with Edit and Delete for those who can
 * manage it. Deleting a thread's first post deletes the whole thread.
 */
export function PostBody({
  postId,
  content,
  thread,
  canManage,
  children,
}: {
  postId: number;
  content: string;
  thread: ForumThread;
  canManage: boolean;
  children: ReactNode;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(content);
  const { error, pending, run } = useAction();
  const isFirst = postId === thread.thread_post_id;

  function onSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    run(() => editPost(postId, draft.trim()), () => setEditing(false));
  }

  if (editing) {
    return (
      <form className={styles.editor} onSubmit={onSave} aria-label="Edit post">
        <textarea
          className={`${styles.input} ${styles.textarea}`}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          maxLength={MAX_CONTENT}
          rows={8}
          aria-label="Post (Markdown supported)"
          required
          autoFocus
        />
        <div className={styles.composerActions}>
          <button type="submit" className={styles.primary} disabled={pending || !draft.trim()}>
            {pending ? "Saving..." : "Save"}
          </button>
          <button
            type="button"
            className={styles.secondary}
            onClick={() => {
              setDraft(content);
              setEditing(false);
            }}
            disabled={pending}
          >
            Cancel
          </button>
          <ErrorText error={error} />
        </div>
      </form>
    );
  }

  return (
    <>
      {children}
      {canManage && (
        <div className={styles.tools} aria-label="Post tools">
          <button
            type="button"
            className={styles.tool}
            onClick={() => {
              setDraft(content);
              setEditing(true);
            }}
            disabled={pending}
          >
            Edit
          </button>
          <DeleteButton
            label="Delete"
            question={isFirst ? "This is the thread's first post. Delete the whole thread?" : "Delete this post?"}
            pending={pending}
            onConfirm={() => run(() => (isFirst ? deleteThread(thread.id, thread.category_id) : deletePost(postId)))}
          />
          <ErrorText error={error} />
        </div>
      )}
    </>
  );
}
