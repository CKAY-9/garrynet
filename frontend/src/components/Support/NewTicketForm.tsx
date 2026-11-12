"use client";

import { useState, type FormEvent } from "react";
import { AttachmentPicker, useAttachments } from "@/components/Upload";
import { UserPicker } from "@/components/UserPicker";
import { MAX_MESSAGE_ATTACHMENTS } from "@/lib/files";
import {
  MAX_ASSOCIATED_USERS,
  MAX_CONTENT,
  MAX_SUBJECT,
  MAX_TITLE,
  ticketCategories,
  ticketCategoryLabels,
} from "@/lib/supportEnums";
import type { TicketCategory, UserRef } from "@/lib/types";
import { createTicket } from "./actions";
import { ErrorText, useAction } from "./TicketTools";
import styles from "./Support.module.scss";

/** /support/new: title, subject, category, the users involved and the first message. */
export function NewTicketForm() {
  const [title, setTitle] = useState("");
  const [subject, setSubject] = useState("");
  const [category, setCategory] = useState<TicketCategory>("OTHER");
  const [associated, setAssociated] = useState<UserRef[]>([]);
  const [content, setContent] = useState("");
  const { error, pending, run } = useAction();
  const attachments = useAttachments(MAX_MESSAGE_ATTACHMENTS);

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    // Redirects to the new ticket on success.
    run(() =>
      createTicket({
        title: title.trim(),
        subject: subject.trim(),
        category,
        associated_user_ids: associated.map((u) => u.id),
        content: content.trim(),
        attachments: attachments.ids,
      }),
    );
  }

  return (
    <form className={styles.composer} onSubmit={onSubmit} aria-label="New ticket">
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
        <span className={styles.label}>Subject</span>
        <input
          className={styles.input}
          value={subject}
          onChange={(e) => setSubject(e.target.value)}
          maxLength={MAX_SUBJECT}
          placeholder="A one-line summary, shown in ticket lists"
          required
        />
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Category</span>
        <select className={styles.input} value={category} onChange={(e) => setCategory(e.target.value as TicketCategory)}>
          {ticketCategories.map((c) => (
            <option key={c} value={c}>
              {ticketCategoryLabels[c]}
            </option>
          ))}
        </select>
      </label>
      <div className={styles.field}>
        <span className={styles.label}>Users involved (optional)</span>
        <UserPicker
          selected={associated}
          onChange={setAssociated}
          max={MAX_ASSOCIATED_USERS}
          hint="For user reports and the like: pick the users this ticket is about."
        />
      </div>
      <label className={styles.field}>
        <span className={styles.label}>Message (Markdown supported)</span>
        <textarea
          className={`${styles.input} ${styles.textarea}`}
          value={content}
          onChange={(e) => setContent(e.target.value)}
          maxLength={MAX_CONTENT}
          rows={8}
          placeholder="Describe the problem, suggestion or report in as much detail as you can."
          required
        />
      </label>
      <AttachmentPicker {...attachments.picker} disabled={pending} />
      <div className={styles.composerActions}>
        <button
          type="submit"
          className={styles.primary}
          disabled={pending || attachments.busy || !title.trim() || !subject.trim() || !content.trim()}
        >
          {pending ? "Opening..." : "Open ticket"}
        </button>
        <ErrorText error={error} />
      </div>
    </form>
  );
}
