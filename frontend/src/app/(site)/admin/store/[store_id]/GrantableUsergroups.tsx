"use client";

import { useState, useTransition, type FormEvent } from "react";
import { MAX_USERGROUP } from "@/lib/storeEnums";
import form from "../../community/editor.module.scss";
import { setGrantableUsergroups } from "../actions";
import styles from "../store.module.scss";

/**
 * The Garry's Mod usergroups a store's items may grant. Editable with Elevated Control
 * (the backend checks too); everyone else sees the list.
 */
export function GrantableUsergroups({ storeId, groups, canEdit }: { storeId: number; groups: string[]; canEdit: boolean }) {
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function save(next: string[]) {
    startTransition(async () => {
      const res = await setGrantableUsergroups(storeId, next);
      if (res.ok) {
        setError(null);
        setDraft("");
      } else {
        setError(res.message);
      }
    });
  }

  function onAdd(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const group = draft.trim();
    if (!group || groups.includes(group)) return;
    save([...groups, group]);
  }

  return (
    <div className={form.group}>
      <p className={styles.panelText}>
        Items can only grant usergroups on this list, so a store manager can&apos;t sell <code>superadmin</code>.
        {canEdit ? " Only users with Elevated Control can change it." : " Ask a user with Elevated Control to change it."}
      </p>
      {groups.length === 0 ? (
        <p className={styles.empty}>No usergroups can be granted yet, so items can&apos;t have usergroup rewards.</p>
      ) : (
        <ul className={styles.chips} aria-label="Grantable usergroups">
          {groups.map((g) => (
            <li key={g} className={`${styles.chip} ${canEdit ? "" : styles.chipStatic}`}>
              {g}
              {canEdit && (
                <button
                  type="button"
                  onClick={() => save(groups.filter((o) => o !== g))}
                  disabled={pending}
                  aria-label={`Stop allowing ${g}`}
                >
                  ×
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
      {canEdit && (
        <form className={styles.chipRow} onSubmit={onAdd}>
          <input
            className={form.input}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            maxLength={MAX_USERGROUP}
            pattern="\S+"
            placeholder="vip"
            aria-label="Usergroup to allow"
          />
          <button type="submit" className={form.save} disabled={pending || !draft.trim()}>
            {pending ? "Saving..." : "Allow"}
          </button>
        </form>
      )}
      {error && (
        <p className={`${form.message} ${form.error}`} role="status">
          {error}
        </p>
      )}
    </div>
  );
}
