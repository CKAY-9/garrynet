"use client";

import { useState, useTransition } from "react";
import Link from "next/link";
import type { MassRefundResult } from "@/lib/types";
import form from "../../community/editor.module.scss";
import { refundUndelivered } from "../../store/actions";
import styles from "../servers.module.scss";

function purchases(n: number): string {
  return `${n} ${n === 1 ? "purchase" : "purchases"}`;
}

/**
 * Paid purchases from the server's store that haven't been delivered, which keep the server
 * from being deleted, with a way to refund them all for users with Store Finance (the
 * backend checks too). Renders nothing while there are none, unless it just refunded some.
 */
export function UndeliveredPurchases({
  serverId,
  count,
  storeId,
  canRefund,
}: {
  serverId: number;
  count: number;
  storeId: number | null;
  canRefund: boolean;
}) {
  const [confirming, setConfirming] = useState(false);
  const [result, setResult] = useState<MassRefundResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  if (count === 0 && !result) return null;

  function refund() {
    startTransition(async () => {
      const res = await refundUndelivered(serverId);
      setConfirming(false);
      if (res.ok) {
        setResult(res.data);
        setError(null);
      } else {
        setError(res.message);
      }
    });
  }

  return (
    <div className={`${form.group} ${styles.dangerGroup}`}>
      {count > 0 ? (
        <p className={styles.panelText}>
          <strong>{purchases(count)}</strong> from this server&apos;s store {count === 1 ? "hasn't" : "haven't"} been
          delivered in-game yet. The server can&apos;t be deleted until {count === 1 ? "it's" : "they're"} delivered or
          refunded.
        </p>
      ) : (
        <p className={styles.panelText}>Every paid purchase from this server&apos;s store has been settled.</p>
      )}

      {result && <RefundSummary result={result} />}

      {count > 0 && !canRefund && (
        <p className={styles.panelText}>Someone with Store Finance can refund them from this page.</p>
      )}

      {count > 0 && canRefund &&
        (confirming ? (
          <div className={styles.confirm} role="alert">
            <p>
              Refund {count === 1 ? "this purchase" : `all ${purchases(count)}`} in full through Stripe? Each buyer is
              told. A purchase delivered in the meantime is left alone.
            </p>
            <div className={styles.inlineActions}>
              <button type="button" className={styles.danger} onClick={refund} disabled={pending}>
                {pending ? "Refunding..." : `Refund ${purchases(count)}`}
              </button>
              <button type="button" className={form.discard} onClick={() => setConfirming(false)} disabled={pending}>
                Cancel
              </button>
            </div>
          </div>
        ) : (
          <div className={styles.inlineActions}>
            <button type="button" className={styles.danger} onClick={() => setConfirming(true)}>
              Refund undelivered purchases
            </button>
            {storeId !== null && <Link href={`/admin/store/receipts?store=${storeId}`}>See the store&apos;s receipts</Link>}
            {error && (
              <p className={`${form.message} ${form.error}`} role="status">
                {error}
              </p>
            )}
          </div>
        ))}
    </div>
  );
}

function RefundSummary({ result }: { result: MassRefundResult }) {
  const { refunded, failed, remaining } = result;
  // Left over without having failed: the run stopped to stay within its time limit.
  const unattempted = remaining - failed.length;
  return (
    <div className={form.message} role="status">
      <p className={styles.panelText}>
        Refunded {purchases(refunded)}.
        {remaining === 0 && " The server can be deleted now."}
        {unattempted > 0 && ` ${purchases(unattempted)} still to go; refund again to continue.`}
      </p>
      {failed.length > 0 && (
        <ul className={form.error}>
          {failed.map((f) => (
            <li key={f.receipt_id}>
              Receipt #{f.receipt_id}: {f.error}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
