"use client";

import { useState, useTransition } from "react";
import form from "../../community/editor.module.scss";
import { retryFulfillment } from "../actions";
import styles from "../store.module.scss";

/**
 * Offers a failed delivery to the server's addon again with a fresh set of attempts. Store
 * Finance only (the backend checks too).
 */
export function RetryButton({ fulfillmentId }: { fulfillmentId: number }) {
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onClick() {
    startTransition(async () => {
      const res = await retryFulfillment(fulfillmentId);
      setError(res.ok ? null : res.message);
    });
  }

  return (
    <span className={styles.tools}>
      <button type="button" className={styles.tool} onClick={onClick} disabled={pending}>
        {pending ? "Retrying..." : "Retry delivery"}
      </button>
      {error && <span className={form.error}>{error}</span>}
    </span>
  );
}
