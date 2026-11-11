import { Suspense } from "react";
import Form from "next/form";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { FulfillmentBadge, ReceiptStatusBadge, When } from "@/components/Store";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import { getAdminStores, getReceipts, readReceiptFilters, type SearchParams } from "@/lib/store";
import { formatMoney, receiptStatuses, receiptStatusLabels, storeName } from "@/lib/storeEnums";
import type { ReceiptFilters, StoreWithServer } from "@/lib/types";
import styles from "../../admin.module.scss";
import form from "../../community/editor.module.scss";
import list from "../store.module.scss";
import { RefundButton } from "./RefundButton";
import { RetryButton } from "./RetryButton";

export default function ReceiptsPage({ searchParams }: PageProps<"/admin/store/receipts">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <Receipts searchParams={searchParams} />
    </Suspense>
  );
}

/** /admin/store/receipts: every receipt, with refunds. Requires Store Finance. */
async function Receipts({ searchParams }: { searchParams: SearchParams }) {
  await requirePermission(Permission.AdminAccess | Permission.StoreFinance);
  const filters = await readReceiptFilters(searchParams);
  const [receipts, stores] = await Promise.all([getReceipts(filters), getAdminStores()]);
  const filtered = Object.values(filters).some((v) => v?.trim());
  const totals = new Map<string, number>();
  for (const r of receipts) {
    if (r.status === "PAID") totals.set(r.currency, (totals.get(r.currency) ?? 0) + r.price);
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
            {filtered ? " match" : " (latest 500)"}
            {totals.size > 0 &&
              ` · paid: ${[...totals].map(([currency, amount]) => formatMoney(amount, currency)).join(", ")}`}
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
                          </span>
                        )}
                      </span>
                    </td>
                    <td>{r.status === "PAID" && <RefundButton receiptId={r.id} amount={formatMoney(r.price, r.currency)} />}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
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
