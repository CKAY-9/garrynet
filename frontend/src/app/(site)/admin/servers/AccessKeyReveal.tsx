"use client";

import { useState } from "react";
import styles from "./servers.module.scss";

/** A freshly generated access key, with a copy button. It can't be shown again. */
export function AccessKeyReveal({ accessKey }: { accessKey: string }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(accessKey);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard blocked (e.g. insecure origin); the key is still selectable.
    }
  }

  return (
    <div className={styles.keyReveal} role="status">
      <p className={styles.keyWarning}>
        Copy this access key now. It won&apos;t be shown again; if you lose it, regenerate it.
      </p>
      <div className={styles.keyRow}>
        <input
          className={styles.keyInput}
          value={accessKey}
          readOnly
          onFocus={(e) => e.currentTarget.select()}
          aria-label="Access key"
          spellCheck={false}
        />
        <button type="button" className={styles.copy} onClick={copy}>
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <p className={styles.keyHint}>
        On the server, run <code>!gnet</code> as a superadmin and paste the key to link it to GarryNet.
      </p>
    </div>
  );
}
