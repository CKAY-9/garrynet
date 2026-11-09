"use client";

import { useState, useTransition, type FormEvent } from "react";
import type { Server } from "@/lib/types";
import form from "../../community/editor.module.scss";
import { ServerFields, toDetails, toDraft, type ServerDraft } from "../ServerForm";
import { updateServer } from "../actions";

type Status = { result: "idle" } | { result: "saved" } | { result: "error"; message: string };

/** The server's editable details. Changes apply when saved. */
export function ServerEditor({ server }: { server: Server }) {
  const [saved, setSaved] = useState(() => toDetails(toDraft(server)));
  const [draft, setDraft] = useState(() => toDraft(server));
  const [status, setStatus] = useState<Status>({ result: "idle" });
  const [saving, startSaving] = useTransition();
  const details = toDetails(draft);
  const changed = JSON.stringify(details) !== JSON.stringify(saved);

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startSaving(async () => {
      const res = await updateServer(server.id, details);
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
        <ServerFields draft={draft} onChange={(patch: Partial<ServerDraft>) => setDraft((d) => ({ ...d, ...patch }))} />
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
