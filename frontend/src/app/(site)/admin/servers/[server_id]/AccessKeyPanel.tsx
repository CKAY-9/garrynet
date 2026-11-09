"use client";

import { useState, useTransition } from "react";
import type { Server } from "@/lib/types";
import form from "../../community/editor.module.scss";
import { AccessKeyReveal } from "../AccessKeyReveal";
import { regenerateAccessKey } from "../actions";
import styles from "../servers.module.scss";

/**
 * Regenerates the access key, for when the server and GarryNet fall out of sync or the key
 * was lost. Asks for confirmation first, since it unlinks a linked server.
 */
export function AccessKeyPanel({ server }: { server: Server }) {
  const [confirming, setConfirming] = useState(false);
  const [accessKey, setAccessKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function regenerate() {
    startTransition(async () => {
      const res = await regenerateAccessKey(server.id);
      setConfirming(false);
      if (res.ok) {
        setAccessKey(res.data.access_key);
        setError(null);
      } else {
        setError(res.message);
      }
    });
  }

  return (
    <div className={form.group}>
      <p className={styles.panelText}>
        {server.linked
          ? "This server is linked. Its access key has been used and replaced with a rolling token."
          : "This server isn't linked yet. Paste its access key into the GarryNet addon (!gnet) to link it."}{" "}
        Keys are only shown once; regenerate one if it was lost or the server falls out of sync.
      </p>

      {accessKey && <AccessKeyReveal accessKey={accessKey} />}

      {confirming ? (
        <div className={styles.confirm} role="alert">
          <p>
            {server.linked
              ? `${server.name} will be unlinked until it links again with the new key.`
              : "The current key will stop working."}{" "}
            Continue?
          </p>
          <div className={styles.inlineActions}>
            <button type="button" className={styles.danger} onClick={regenerate} disabled={pending}>
              {pending ? "Regenerating..." : "Regenerate key"}
            </button>
            <button type="button" className={form.discard} onClick={() => setConfirming(false)} disabled={pending}>
              Cancel
            </button>
          </div>
        </div>
      ) : (
        <div className={styles.inlineActions}>
          <button type="button" className={form.discard} onClick={() => setConfirming(true)}>
            Regenerate access key
          </button>
          {error && <p className={`${form.message} ${form.error}`} role="status">{error}</p>}
        </div>
      )}
    </div>
  );
}
