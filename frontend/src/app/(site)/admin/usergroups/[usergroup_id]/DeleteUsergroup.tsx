"use client";

import { useState, useTransition } from "react";
import type { Usergroup } from "@/lib/types";
import form from "../../community/editor.module.scss";
import servers from "../../servers/servers.module.scss";
import { deleteUsergroup } from "../actions";

/** Deletes the usergroup after the admin types its name. */
export function DeleteUsergroup({ group }: { group: Usergroup }) {
  const [typed, setTyped] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const matches = typed.trim() === group.name;
  const members = group.members.length;

  function onDelete() {
    startTransition(async () => {
      // Redirects to the usergroup list on success.
      const res = await deleteUsergroup(group.id);
      if (!res.ok) setError(res.message);
    });
  }

  return (
    <div className={`${form.group} ${servers.dangerGroup}`}>
      <p className={servers.panelText}>
        Deleting a usergroup removes it from its {members} {members === 1 ? "member" : "members"}, who lose its
        permissions. This can&apos;t be undone.
      </p>
      <label className={form.field}>
        <span className={form.label}>
          Type <strong>{group.name}</strong> to confirm
        </span>
        <input className={form.input} value={typed} onChange={(e) => setTyped(e.target.value)} autoComplete="off" />
      </label>
      <div className={servers.inlineActions}>
        <button type="button" className={servers.danger} onClick={onDelete} disabled={!matches || pending}>
          {pending ? "Deleting..." : "Delete usergroup"}
        </button>
        {error && <p className={`${form.message} ${form.error}`} role="status">{error}</p>}
      </div>
    </div>
  );
}
