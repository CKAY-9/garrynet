"use client";

import { useState, useTransition, type FormEvent } from "react";
import { isDefaultGroup } from "@/lib/permissions";
import type { Usergroup } from "@/lib/types";
import form from "../../community/editor.module.scss";
import { UsergroupFields, toDetails, toDraft, type UsergroupDraft } from "../UsergroupForm";
import { updateUsergroup } from "../actions";

type Status = { result: "idle" } | { result: "saved" } | { result: "error"; message: string };

/** The usergroup's editable details. Changes apply when saved. */
export function UsergroupEditor({ group, held }: { group: Usergroup; held: number }) {
  const [saved, setSaved] = useState(() => toDetails(toDraft(group)));
  const [draft, setDraft] = useState(() => toDraft(group));
  const [status, setStatus] = useState<Status>({ result: "idle" });
  const [saving, startSaving] = useTransition();
  const details = toDetails(draft);
  const changed = JSON.stringify(details) !== JSON.stringify(saved);

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startSaving(async () => {
      const res = await updateUsergroup(group.id, details);
      if (res.ok) {
        // Show the details as the backend normalized them.
        const next = toDraft(res.data);
        setSaved(toDetails(next));
        setDraft(next);
        setStatus({ result: "saved" });
      } else {
        setStatus({ result: "error", message: res.message });
      }
    });
  }

  return (
    <form className={form.editor} onSubmit={onSubmit}>
      <fieldset className={form.group}>
        <legend className={form.legend}>Details</legend>
        <UsergroupFields
          draft={draft}
          onChange={(patch: Partial<UsergroupDraft>) => setDraft((d) => ({ ...d, ...patch }))}
          held={held}
          lockName={isDefaultGroup(saved.name)}
        />
      </fieldset>

      <div className={form.actions}>
        <button type="submit" className={form.save} disabled={!changed || saving}>
          {saving ? "Saving..." : "Save"}
        </button>
        <button
          type="button"
          className={form.discard}
          onClick={() => {
            setDraft(toDraft(saved));
            setStatus({ result: "idle" });
          }}
          disabled={!changed || saving}
        >
          Discard changes
        </button>
        <p className={form.message} role="status">
          {status.result === "error" && <span className={form.error}>{status.message}</span>}
          {status.result === "saved" && !changed && "Saved."}
          {status.result !== "error" && changed && !saving && "Unsaved changes."}
        </p>
      </div>
    </form>
  );
}
