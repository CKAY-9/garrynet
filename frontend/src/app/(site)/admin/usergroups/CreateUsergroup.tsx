"use client";

import { useState, useTransition, type FormEvent } from "react";
import form from "../community/editor.module.scss";
import servers from "../servers/servers.module.scss";
import { EMPTY_DRAFT, UsergroupFields, toDetails, type UsergroupDraft } from "./UsergroupForm";
import { createUsergroup } from "./actions";

/** Creates a usergroup, then opens its page. */
export function CreateUsergroup({ held }: { held: number }) {
  const [draft, setDraft] = useState<UsergroupDraft>(EMPTY_DRAFT);
  const [error, setError] = useState<string | null>(null);
  const [saving, startSaving] = useTransition();

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startSaving(async () => {
      // Redirects to the new group's page on success.
      const res = await createUsergroup(toDetails(draft));
      if (!res.ok) setError(res.message);
    });
  }

  return (
    <form className={form.group} onSubmit={onSubmit}>
      <UsergroupFields draft={draft} onChange={(patch) => setDraft((d) => ({ ...d, ...patch }))} held={held} />
      <div className={servers.inlineActions}>
        <button type="submit" className={form.save} disabled={saving || !draft.name.trim()}>
          {saving ? "Creating..." : "Create usergroup"}
        </button>
        <p className={form.message} role="status">
          {error ? <span className={form.error}>{error}</span> : "Members can be added from the backend shell."}
        </p>
      </div>
    </form>
  );
}
