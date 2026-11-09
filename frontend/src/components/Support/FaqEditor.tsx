"use client";

import { useState, type FormEvent, type ReactNode } from "react";
import { faqCategories, faqCategoryLabels, MAX_CONTENT, MAX_SUBJECT, MAX_TITLE } from "@/lib/supportEnums";
import type { Faq, FaqCategory } from "@/lib/types";
import { createFaq, deleteFaq, updateFaq } from "./actions";
import { ErrorText, useAction } from "./TicketTools";
import styles from "./Support.module.scss";

/**
 * Creates an FAQ, or edits `faq` when given. Support Team only (the backend checks too).
 * Creating opens the new FAQ; editing calls `onDone` once saved.
 */
export function FaqForm({ faq, onDone }: { faq?: Faq; onDone?: () => void }) {
  const [title, setTitle] = useState(faq?.title ?? "");
  const [subject, setSubject] = useState(faq?.subject ?? "");
  const [category, setCategory] = useState<FaqCategory>(faq?.category ?? "GENERAL");
  const [description, setDescription] = useState(faq?.description ?? "");
  const { error, pending, run } = useAction();

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const details = { title: title.trim(), subject: subject.trim(), category, description: description.trim() };
    run(() => (faq ? updateFaq(faq.id, details) : createFaq(details)), onDone);
  }

  return (
    <form className={styles.composer} onSubmit={onSubmit} aria-label={faq ? "Edit FAQ" : "New FAQ"}>
      <label className={styles.field}>
        <span className={styles.label}>Title</span>
        <input className={styles.input} value={title} onChange={(e) => setTitle(e.target.value)} maxLength={MAX_TITLE} required autoFocus />
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Subject</span>
        <input
          className={styles.input}
          value={subject}
          onChange={(e) => setSubject(e.target.value)}
          maxLength={MAX_SUBJECT}
          placeholder="A one-line summary, shown in the FAQ list"
          required
        />
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Category</span>
        <select className={styles.input} value={category} onChange={(e) => setCategory(e.target.value as FaqCategory)}>
          {faqCategories.map((c) => (
            <option key={c} value={c}>
              {faqCategoryLabels[c]}
            </option>
          ))}
        </select>
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Answer (Markdown supported)</span>
        <textarea
          className={`${styles.input} ${styles.textarea}`}
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          maxLength={MAX_CONTENT}
          rows={12}
          required
        />
      </label>
      <div className={styles.composerActions}>
        <button type="submit" className={styles.primary} disabled={pending || !title.trim() || !subject.trim() || !description.trim()}>
          {pending ? "Saving..." : faq ? "Save changes" : "Create FAQ"}
        </button>
        {onDone && (
          <button type="button" className={styles.secondary} onClick={onDone} disabled={pending}>
            Cancel
          </button>
        )}
        <ErrorText error={error} />
      </div>
    </form>
  );
}

/** An FAQ's answer (`children`, already rendered) with Edit and Delete for the Support Team. */
export function FaqBody({ faq, canManage, children }: { faq: Faq; canManage: boolean; children: ReactNode }) {
  const [editing, setEditing] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const { error, pending, run } = useAction();

  if (editing) return <FaqForm key={faq.last_updated} faq={faq} onDone={() => setEditing(false)} />;

  return (
    <>
      {children}
      {canManage && (
        <div className={styles.tools} aria-label="FAQ tools">
          <button type="button" className={styles.tool} onClick={() => setEditing(true)} disabled={pending}>
            Edit
          </button>
          {confirming ? (
            <span className={styles.confirm} role="group" aria-label="Delete this FAQ?">
              <span>Delete this FAQ?</span>
              <button
                type="button"
                className={`${styles.tool} ${styles.dangerTool}`}
                onClick={() => run(() => deleteFaq(faq.id))}
                disabled={pending}
              >
                {pending ? "Deleting..." : "Delete"}
              </button>
              <button type="button" className={styles.tool} onClick={() => setConfirming(false)} disabled={pending}>
                Cancel
              </button>
            </span>
          ) : (
            <button type="button" className={styles.tool} onClick={() => setConfirming(true)} disabled={pending}>
              Delete
            </button>
          )}
          <ErrorText error={error} />
        </div>
      )}
    </>
  );
}
