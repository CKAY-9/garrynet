"use client";

import { useState, useTransition, type FormEvent, type ReactNode } from "react";
import { AttachmentPicker, useAttachments } from "@/components/Upload";
import type { ActionResult } from "@/lib/actionResult";
import { MAX_MESSAGE_ATTACHMENTS } from "@/lib/files";
import { isDone, MAX_CONTENT } from "@/lib/supportEnums";
import type { SupportMessage, SupportStatus, SupportTicket } from "@/lib/types";
import { editMessage, replyToTicket, setTicketStatus } from "./actions";
import styles from "./Support.module.scss";

/** Runs a server action, keeping its error message for display. */
export function useAction() {
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

export function ErrorText({ error }: { error: string | null }) {
  if (!error) return null;
  return (
    <p className={styles.error} role="status">
      {error}
    </p>
  );
}

/** Reply box at the bottom of a ticket. The Support Team can send internal notes instead. */
export function ReplyForm({ ticketId, support }: { ticketId: number; support: boolean }) {
  const [content, setContent] = useState("");
  const [internal, setInternal] = useState(false);
  const { error, pending, run } = useAction();
  const attachments = useAttachments(MAX_MESSAGE_ATTACHMENTS);

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    run(
      () => replyToTicket(ticketId, content.trim(), support && internal, attachments.ids),
      () => {
        setContent("");
        attachments.clear();
      },
    );
  }

  return (
    <form className={styles.composer} onSubmit={onSubmit} aria-label="Reply">
      <textarea
        className={`${styles.input} ${styles.textarea}`}
        value={content}
        onChange={(e) => setContent(e.target.value)}
        maxLength={MAX_CONTENT}
        rows={5}
        placeholder={internal ? "Write a note for the support team..." : "Write a reply... (Markdown supported)"}
        aria-label="Reply"
        required
      />
      <AttachmentPicker {...attachments.picker} disabled={pending} />
      <div className={styles.composerActions}>
        <button type="submit" className={styles.primary} disabled={pending || attachments.busy || !content.trim()}>
          {pending ? "Sending..." : internal ? "Add note" : "Reply"}
        </button>
        {support && (
          <label className={styles.checkbox}>
            <input type="checkbox" checked={internal} onChange={(e) => setInternal(e.target.checked)} />
            Internal note (hidden from the user)
          </label>
        )}
        <ErrorText error={error} />
      </div>
    </form>
  );
}

/** Resolve, close or re-open a ticket. Support Team only. */
export function StatusMenu({ ticket }: { ticket: SupportTicket }) {
  const { error, pending, run } = useAction();
  const set = (status: SupportStatus) => run(() => setTicketStatus(ticket.id, status));
  return (
    <div className={styles.tools} role="group" aria-label="Ticket status">
      {ticket.status !== "RESOLVED" && (
        <button type="button" className={`${styles.tool} ${styles.successTool}`} onClick={() => set("RESOLVED")} disabled={pending}>
          Mark resolved
        </button>
      )}
      {ticket.status !== "CLOSED" && (
        <button type="button" className={`${styles.tool} ${styles.dangerTool}`} onClick={() => set("CLOSED")} disabled={pending}>
          Close
        </button>
      )}
      {isDone(ticket.status) && (
        <button type="button" className={styles.tool} onClick={() => set("AWAITING_USER")} disabled={pending}>
          Re-open
        </button>
      )}
      <ErrorText error={error} />
    </div>
  );
}

/**
 * A message's content (`children`, already rendered) with an Edit button for its author.
 * Messages can only be edited once, so the button disappears afterwards.
 */
export function MessageBody({
  message,
  canEdit,
  children,
}: {
  message: SupportMessage;
  canEdit: boolean;
  children: ReactNode;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(message.content);
  const { error, pending, run } = useAction();

  function onSave(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    run(
      () => editMessage(message.id, draft.trim()),
      () => setEditing(false),
    );
  }

  if (editing) {
    return (
      <form className={styles.editor} onSubmit={onSave} aria-label="Edit message">
        <textarea
          className={`${styles.input} ${styles.textarea}`}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          maxLength={MAX_CONTENT}
          rows={8}
          aria-label="Message (Markdown supported)"
          required
          autoFocus
        />
        <p className={styles.hint}>Messages can only be edited once. The previous version stays visible.</p>
        <div className={styles.composerActions}>
          <button type="submit" className={styles.primary} disabled={pending || !draft.trim()}>
            {pending ? "Saving..." : "Save"}
          </button>
          <button
            type="button"
            className={styles.secondary}
            onClick={() => {
              setDraft(message.content);
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
      {canEdit && (
        <div className={styles.tools} aria-label="Message tools">
          <button
            type="button"
            className={styles.tool}
            onClick={() => {
              setDraft(message.content);
              setEditing(true);
            }}
            disabled={pending}
          >
            Edit
          </button>
          <ErrorText error={error} />
        </div>
      )}
    </>
  );
}
