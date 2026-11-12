"use client";

import { useEffect, useState, type ReactNode } from "react";
import { Avatar } from "@/components/Avatar";
import type { UserRef } from "@/lib/types";
import { findUsers } from "./actions";
import styles from "./UserPicker.module.scss";

type Common = {
  /** Shown under the search box. */
  hint?: ReactNode;
  /** Users never offered, e.g. the current user. */
  exclude?: number[];
  autoFocus?: boolean;
  /** Disables the search box. */
  disabled?: boolean;
};

/** Pick several users, shown as removable chips. */
type MultipleProps = Common & {
  mode?: "multiple";
  selected: UserRef[];
  onChange: (users: UserRef[]) => void;
  max: number;
};

/** Pick one user; the parent decides what happens next (e.g. opening their chat). */
type SingleProps = Common & {
  mode: "single";
  onSelect: (user: UserRef) => void;
};

/**
 * Searches users by name or numeric id as the user types. In `multiple` mode (the default)
 * the picked users show as chips; in `single` mode each pick is handed to `onSelect`.
 */
export function UserPicker(props: MultipleProps | SingleProps) {
  const { hint, exclude = [], autoFocus = false, disabled = false } = props;
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<UserRef[]>([]);
  const [searching, setSearching] = useState(false);
  const [failed, setFailed] = useState(false);

  // Search a moment after typing stops.
  useEffect(() => {
    const q = query.trim();
    if (!q) return;
    let cancelled = false;
    const timer = setTimeout(async () => {
      setSearching(true);
      const res = await findUsers(q);
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

  const selected = props.mode === "single" ? [] : props.selected;
  const max = props.mode === "single" ? 1 : props.max;
  const full = props.mode !== "single" && selected.length >= max;
  const hidden = new Set([...exclude, ...selected.map((u) => u.id)]);
  const visible = query.trim() ? results.filter((u) => !hidden.has(u.id)) : [];

  function pick(user: UserRef) {
    if (props.mode === "single") {
      props.onSelect(user);
    } else {
      if (full) return;
      props.onChange([...selected, user]);
    }
    setQuery("");
  }

  return (
    <div className={styles.picker}>
      {props.mode !== "single" && selected.length > 0 && (
        <ul className={styles.chips} aria-label="Selected users">
          {selected.map((u) => (
            <li key={u.id} className={styles.chip}>
              <Avatar name={u.username} url={u.avatar_url || null} size={20} shape="circle" />
              {u.username}
              <button
                type="button"
                aria-label={`Remove ${u.username}`}
                onClick={() => props.onChange(selected.filter((s) => s.id !== u.id))}
              >
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
        placeholder={full ? `Up to ${max} users` : "Search users by name or id..."}
        aria-label="Search users"
        disabled={disabled || full}
        autoComplete="off"
        autoFocus={autoFocus}
      />
      {query.trim() && (
        <ul className={styles.results} aria-label="Matching users">
          {visible.map((u) => (
            <li key={u.id}>
              <button type="button" onClick={() => pick(u)}>
                <Avatar name={u.username} url={u.avatar_url || null} size={24} shape="circle" />
                {u.username} <span className={styles.id}>#{u.id}</span>
              </button>
            </li>
          ))}
          {visible.length === 0 && <li>{searching ? "Searching..." : failed ? "Couldn't search users." : "No users found."}</li>}
        </ul>
      )}
      {hint && <p className={styles.hint}>{hint}</p>}
    </div>
  );
}
