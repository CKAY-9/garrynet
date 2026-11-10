"use client";

import { useState, useTransition } from "react";
import form from "../../community/editor.module.scss";
import { refundReceipt } from "../actions";
import styles from "../store.module.scss";

/** Refunds a receipt in full after a second click. Store Finance only (the backend checks too). */
export function RefundButton({ receiptId, amount }: { receiptId: number; amount: string }) {
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onClick() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    startTransition(async () => {
      const res = await refundReceipt(receiptId);
      if (!res.ok) {
        setError(res.message);
        setConfirming(false);
      }
    });
  }

  return (
    <span className={styles.tools}>
      <button
        type="button"
        className={confirming ? styles.dangerConfirm : styles.danger}
        onClick={onClick}
        onBlur={() => setConfirming(false)}
        disabled={pending}
        aria-label={confirming ? `Confirm refunding ${amount}` : `Refund ${amount}`}
      >
        {pending ? "Refunding..." : confirming ? `Refund ${amount}?` : "Refund"}
      </button>
      {error && <span className={form.error}>{error}</span>}
    </span>
  );
}
