"use client";

import { useEffect, useState, type FormEvent } from "react";
import { Avatar } from "@/components/Avatar";
import { AttachmentPicker, useAttachments } from "@/components/Upload";
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
import { createTicket, findUsers } from "./actions";
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
        <UserPicker selected={associated} onChange={setAssociated} />
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

/** Searches users by name and lets the user pick several; the picked ones show as chips. */
function UserPicker({ selected, onChange }: { selected: UserRef[]; onChange: (users: UserRef[]) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<UserRef[]>([]);
  const [searching, setSearching] = useState(false);
  const [failed, setFailed] = useState(false);

  // Search a moment after typing stops.
  useEffect(() => {
    const name = query.trim();
    if (!name) return;
    let cancelled = false;
    const timer = setTimeout(async () => {
      setSearching(true);
      const res = await findUsers(name);
      if (cancelled) return;
      setSearching(false);
      setFailed(!res.ok);
      setResults(res.ok ? res.data : []);
    }, 300);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query]);

  const full = selected.length >= MAX_ASSOCIATED_USERS;
  const visible = query.trim() ? results.filter((u) => !selected.some((s) => s.id === u.id)) : [];

  function add(user: UserRef) {
    if (full) return;
    onChange([...selected, user]);
    setQuery("");
  }

  return (
    <div className={styles.picker}>
      {selected.length > 0 && (
        <ul className={styles.chips} aria-label="Selected users">
          {selected.map((u) => (
            <li key={u.id} className={styles.chip}>
              <Avatar name={u.username} url={u.avatar_url || null} size={20} shape="circle" />
              {u.username}
              <button type="button" aria-label={`Remove ${u.username}`} onClick={() => onChange(selected.filter((s) => s.id !== u.id))}>
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
      <input
        type="search"
        className={styles.input}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={full ? `Up to ${MAX_ASSOCIATED_USERS} users` : "Search users by name..."}
        aria-label="Search users"
        disabled={full}
        autoComplete="off"
      />
      {query.trim() && (
        <ul className={styles.results} aria-label="Matching users">
          {visible.map((u) => (
            <li key={u.id}>
              <button type="button" onClick={() => add(u)}>
                <Avatar name={u.username} url={u.avatar_url || null} size={24} shape="circle" />
                {u.username} <span className={styles.id}>#{u.id}</span>
              </button>
            </li>
          ))}
          {visible.length === 0 && <li>{searching ? "Searching..." : failed ? "Couldn't search users." : "No users found."}</li>}
        </ul>
      )}
      <p className={styles.hint}>For user reports and the like: pick the users this ticket is about.</p>
    </div>
  );
}
