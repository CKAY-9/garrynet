import { Suspense } from "react";
import type { Metadata } from "next";
import Form from "next/form";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { FulfillmentBadge, ReceiptStatusBadge, When } from "@/components/Store";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import {
  firstParam,
  getAdminStores,
  getReceipts,
  readReceiptFilters,
  receiptParams,
  type SearchParams,
} from "@/lib/store";
import { formatMoney, receiptStatuses, receiptStatusLabels, storeName } from "@/lib/storeEnums";
import type { ReceiptFilters, ReceiptView, StoreWithServer } from "@/lib/types";
import styles from "../../admin.module.scss";
import form from "../../community/editor.module.scss";
import list from "../store.module.scss";
import { RefundButton } from "./RefundButton";
import { ReinstateButton } from "./ReinstateButton";
import { RetryButton } from "./RetryButton";

export const metadata: Metadata = {
  title: "Receipts",
};

export default function ReceiptsPage({ searchParams }: PageProps<"/admin/store/receipts">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <Receipts searchParams={searchParams} />
    </Suspense>
  );
}

/** How a disputed receipt's dispute stands, by Stripe's status for it once it closed. */
const disputeOutcomes: Record<string, string> = {
  won: "dispute won",
  warning_closed: "inquiry closed",
  lost: "dispute lost",
};

/** Dispute outcomes that let a disputed receipt be reinstated. Must match `DISPUTE_WON` in the backend. */
const disputeWon = ["won", "warning_closed"];

/** Stripe's refund statuses that aren't final, as shown on a receipt. */
const refundUnderWay: Record<string, string> = {
  pending: "refund pending",
  requires_action: "refund awaiting action",
};

/**
 * A refunded receipt whose refund failed or was cancelled after Stripe accepted it: the buyer
 * has neither the money nor the rewards. Like `PaymentRefunds::falls_short` in the backend.
 */
function refundFailed(r: ReceiptView): boolean {
  const refunds = r.refunds;
  return r.status === "REFUNDED" && !!refunds && refunds.amount_charged > 0 && refunds.amount_refunded < refunds.amount_charged;
}

/**
 * What was kept of a receipt's price, as the backend counts income: in proportion to what
 * Stripe didn't refund once it has reported refunds, otherwise all of a paid receipt.
 */
function keptAmount(r: ReceiptView): number {
  if (r.status === "DISPUTED") return 0;
  if (r.refunds && r.refunds.amount_charged > 0) {
    const kept = Math.max(r.refunds.amount_charged - r.refunds.amount_refunded, 0);
    return Math.floor((r.price * kept) / r.refunds.amount_charged);
  }
  return r.status === "PAID" ? r.price : 0;
}

/** /admin/store/receipts: every receipt, a page at a time, with refunds. Requires Store Finance. */
async function Receipts({ searchParams }: { searchParams: SearchParams }) {
  await requirePermission(Permission.AdminAccess | Permission.StoreFinance);
  const filters = await readReceiptFilters(searchParams);
  const cursor = firstParam(await searchParams, "cursor");
  const [page, stores] = await Promise.all([getReceipts(filters, cursor), getAdminStores()]);
  const receipts = page.items;
  const filtered = Object.values(filters).some((v) => v?.trim());
  const totals = new Map<string, number>();
  for (const r of receipts) {
    const kept = keptAmount(r);
    if (kept > 0) totals.set(r.currency, (totals.get(r.currency) ?? 0) + kept);
  }

  return (
    <>
      <Link href="/admin/store" className={list.back}>
        ← All stores
      </Link>
      <section className={styles.section} aria-labelledby="receipts-heading">
        <div className={list.titleRow}>
          <h2 id="receipts-heading" className={styles.sectionTitle}>
            Receipts
          </h2>
          <span className={list.cardMeta}>
            {receipts.length} {receipts.length === 1 ? "receipt" : "receipts"}
            {filtered && " match"}
            {(cursor || page.next_cursor) && " on this page"}
            {totals.size > 0 &&
              ` · net: ${[...totals].map(([currency, amount]) => formatMoney(amount, currency)).join(", ")}`}
          </span>
        </div>
        <ReceiptFilterForm filters={filters} stores={stores} />
        {receipts.length === 0 ? (
          <p className={styles.placeholder}>{filtered ? "No receipts match these filters." : "Nothing has been bought yet."}</p>
        ) : (
          <div className={list.tableWrap}>
            <table className={list.table}>
              <thead>
                <tr>
                  <th scope="col">Receipt</th>
                  <th scope="col">Buyer</th>
                  <th scope="col">Item</th>
                  <th scope="col" className={list.num}>
                    Paid
                  </th>
                  <th scope="col">Status</th>
                  <th scope="col">Delivery</th>
                  <th scope="col">
                    <span className={list.srOnly}>Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {receipts.map((r) => (
                  <tr key={r.id}>
                    <td>
                      <span className={list.cellStack}>
                        <span>#{r.id}</span>
                        <span className={list.cellMeta}>
                          <When iso={r.purchased_at} />
                        </span>
                      </span>
                    </td>
                    <td>
                      <Link href={`/users/${r.buyer.id}`} className={list.buyer}>
                        <Avatar name={r.buyer.username} url={r.buyer.avatar_url || null} size={24} shape="circle" />
                        {r.buyer.username}
                      </Link>
                    </td>
                    <td className={list.wrap}>
                      <span className={list.cellStack}>
                        {r.item_id !== null && r.store_id !== null ? (
                          <Link href={`/admin/store/${r.store_id}/items/${r.item_id}`}>{r.item_name}</Link>
                        ) : (
                          <span>{r.item_name}</span>
                        )}
                        <span className={list.cellMeta}>{r.store_name}</span>
                      </span>
                    </td>
                    <td className={list.num}>
                      <span className={list.cellStack}>
                        <strong>{formatMoney(r.price, r.currency)}</strong>
                        {r.discount_percentage > 0 && (
                          <span className={list.cellMeta}>
                            <s>{formatMoney(r.list_price, r.currency)}</s> · {r.discount_percentage}% off
                          </span>
                        )}
                        {/* Refunds are in what Stripe charged, which includes any tax. */}
                        {r.status === "PAID" && r.refunds && r.refunds.amount_refunded > 0 && (
                          <span className={list.cellMeta}>
                            {formatMoney(r.refunds.amount_refunded, r.currency)} of{" "}
                            {formatMoney(r.refunds.amount_charged, r.currency)} refunded
                          </span>
                        )}
                      </span>
                    </td>
                    <td>
                      <span className={list.cellStack}>
                        <ReceiptStatusBadge status={r.status} />
                        {r.status_changed_at && (
                          <span className={list.cellMeta}>
                            <When iso={r.status_changed_at} />
                          </span>
                        )}
                        {r.status === "DISPUTED" && (
                          <span className={list.cellMeta}>
                            {r.dispute_outcome ? (disputeOutcomes[r.dispute_outcome] ?? r.dispute_outcome) : "dispute open"}
                          </span>
                        )}
                        {refundFailed(r) ? (
                          <>
                            <span className={`${list.badge} ${list.warn}`}>Refund failed</span>
                            <span className={list.cellMeta}>
                              {r.refunds?.latest_failure_reason?.replaceAll("_", " ") ?? r.refunds?.latest_status ?? "not refunded"}
                              {r.refunds && r.refunds.amount_refunded > 0 &&
                                ` · ${formatMoney(r.refunds.amount_refunded, r.currency)} refunded`}
                            </span>
                          </>
                        ) : (
                          r.status === "REFUNDED" &&
                          r.refunds?.latest_status &&
                          refundUnderWay[r.refunds.latest_status] && (
                            <span className={list.cellMeta}>{refundUnderWay[r.refunds.latest_status]}</span>
                          )
                        )}
                      </span>
                    </td>
                    <td>
                      <span className={list.cellStack}>
                        <FulfillmentBadge status={r.fulfillment?.status ?? null} />
                        {r.fulfillment?.status === "FAILED" && r.fulfillment.last_error && (
                          <span className={list.cellMeta} title={r.fulfillment.last_error}>
                            {r.fulfillment.attempts} attempts
                          </span>
                        )}
                        {r.fulfillment?.status === "FAILED" && r.status === "PAID" && (
                          <RetryButton fulfillmentId={r.fulfillment.id} />
                        )}
                        {r.fulfillment?.status === "REVOKED" && (
                          <span className={list.cellMeta}>
                            {r.fulfillment.fulfilled_at
                              ? r.fulfillment.revocation_applied_at
                                ? "rewards removed"
                                : "removal pending"
                              : "never delivered"}
                            {r.fulfillment.money_reclaimed !== null &&
                              ` · ${r.fulfillment.money_reclaimed.toLocaleString()} money taken back`}
                          </span>
                        )}
                      </span>
                    </td>
                    <td>
                      {r.status === "PAID" && <RefundButton receiptId={r.id} amount={formatMoney(r.price, r.currency)} />}
                      {r.status === "DISPUTED" && r.dispute_outcome && disputeWon.includes(r.dispute_outcome) && (
                        <ReinstateButton receiptId={r.id} />
                      )}
                      {refundFailed(r) && r.refunds && (
                        <>
                          <RefundButton
                            receiptId={r.id}
                            amount={formatMoney(r.refunds.amount_charged - r.refunds.amount_refunded, r.currency)}
                            again
                          />
                          <ReinstateButton receiptId={r.id} />
                        </>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        {(cursor || page.next_cursor) && (
          <nav className={list.inlineActions} aria-label="Receipt pages">
            {cursor && <Link href={`/admin/store/receipts?${receiptParams(filters)}`}>← Newest</Link>}
            {page.next_cursor && (
              <Link href={`/admin/store/receipts?${receiptParams(filters, page.next_cursor)}`}>Older receipts →</Link>
            )}
          </nav>
        )}
      </section>
    </>
  );
}

/** Status, store and buyer filters, submitted as a GET form so they live in the URL. */
function ReceiptFilterForm({ filters, stores }: { filters: ReceiptFilters; stores: StoreWithServer[] }) {
  return (
    // Keyed by the filters so the fields reset to the URL's values after each submit.
    <Form key={JSON.stringify(filters)} action="/admin/store/receipts" className={list.filters} role="search">
      <label className={form.field}>
        <span className={form.label}>Status</span>
        <select className={form.input} name="status" defaultValue={filters.status ?? ""}>
          <option value="">Any status</option>
          {receiptStatuses.map((s) => (
            <option key={s} value={s}>
              {receiptStatusLabels[s]}
            </option>
          ))}
        </select>
      </label>
      <label className={form.field}>
        <span className={form.label}>Store</span>
        <select className={form.input} name="store" defaultValue={filters.store ?? ""}>
          <option value="">Any store</option>
          {stores.map((s) => (
            <option key={s.id} value={s.id}>
              {storeName(s, s.server)}
            </option>
          ))}
        </select>
      </label>
      <label className={form.field}>
        <span className={form.label}>Buyer (user id)</span>
        <input className={form.input} name="buyer" defaultValue={filters.buyer ?? ""} inputMode="numeric" pattern="\d*" />
      </label>
      <div className={list.inlineActions}>
        <button className={form.save} type="submit">
          Apply
        </button>
        <Link className={list.clear} href="/admin/store/receipts">
          Clear
        </Link>
      </div>
    </Form>
  );
}
