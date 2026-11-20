"use client";

import { useState, useTransition } from "react";
import form from "../../community/editor.module.scss";
import { reinstateReceipt } from "../actions";
import styles from "../store.module.scss";

/**
 * Reinstates a disputed receipt whose dispute was won, or a refunded one whose refund failed,
 * after a second click: it's paid again and its rewards are delivered again. Store Finance
 * only (the backend checks too).
 */
export function ReinstateButton({ receiptId }: { receiptId: number }) {
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onClick() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    startTransition(async () => {
      const res = await reinstateReceipt(receiptId);
      setConfirming(false);
      setError(res.ok ? null : res.message);
    });
  }

  return (
    <span className={styles.tools}>
      <button
        type="button"
        className={styles.tool}
        onClick={onClick}
        onBlur={() => setConfirming(false)}
        disabled={pending}
        aria-label={confirming ? "Confirm reinstating this purchase" : "Reinstate this purchase"}
      >
        {pending ? "Reinstating..." : confirming ? "Deliver again?" : "Reinstate"}
      </button>
      {error && <span className={form.error}>{error}</span>}
    </span>
  );
}
