"use client";

import { useState, useTransition } from "react";
import type { Server } from "@/lib/types";
import form from "../../community/editor.module.scss";
import { deleteServer } from "../actions";
import styles from "../servers.module.scss";

/**
 * Deletes the server after the admin types its name. `blocked` while paid purchases from its
 * store haven't been delivered (see `UndeliveredPurchases`); the backend refuses then too.
 */
export function DeleteServer({ server, blocked }: { server: Server; blocked: boolean }) {
  const [typed, setTyped] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const matches = typed.trim() === server.name;

  function onDelete() {
    startTransition(async () => {
      // Redirects to the server list on success.
      const res = await deleteServer(server.id);
      if (!res.ok) setError(res.message);
    });
  }

  return (
    <div className={`${form.group} ${styles.dangerGroup}`}>
      <p className={styles.panelText}>
        Deleting a server removes it and every player record GravNet has for it. This can&apos;t be undone.
      </p>
      <label className={form.field}>
        <span className={form.label}>
          Type <strong>{server.name}</strong> to confirm
        </span>
        <input className={form.input} value={typed} onChange={(e) => setTyped(e.target.value)} autoComplete="off" />
      </label>
      <div className={styles.inlineActions}>
        <button type="button" className={styles.danger} onClick={onDelete} disabled={blocked || !matches || pending}>
          {pending ? "Deleting..." : "Delete server"}
        </button>
        {error && <p className={`${form.message} ${form.error}`} role="status">{error}</p>}
      </div>
    </div>
  );
}
