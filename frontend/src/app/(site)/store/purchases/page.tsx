import { Suspense } from "react";
import { unauthorized } from "next/navigation";
import { PurchaseRows, StoreNav } from "@/components/Store";
import { firstParam, getMyPurchases, getStoreViewer, type SearchParams } from "@/lib/store";
import styles from "@/components/Store/Store.module.scss";

export default function PurchasesPage({ searchParams }: PageProps<"/store/purchases">) {
  return (
    // The session and receipts are request-time, so they stream in behind the boundary.
    <Suspense>
      <Purchases searchParams={searchParams} />
    </Suspense>
  );
}

/** /store/purchases: the user's receipts and whether each was delivered. Login required. */
async function Purchases({ searchParams }: { searchParams: SearchParams }) {
  const [viewer, params] = await Promise.all([getStoreViewer(), searchParams]);
  if (!viewer.userId) unauthorized();
  const receipts = await getMyPurchases();
  const justPaid = firstParam(params, "paid") === "1";
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1>My purchases</h1>
        <p>Everything you&apos;ve bought, and whether it has reached you in-game.</p>
      </header>
      <StoreNav loggedIn active="purchases" />
      {justPaid && (
        <p className={`${styles.notice} ${styles.success}`} role="status">
          Thanks, your payment went through. Rewards are delivered the next time you join the server. If the purchase
          isn&apos;t listed yet, refresh in a moment: the payment provider is still confirming it.
        </p>
      )}
      <PurchaseRows receipts={receipts} empty="You haven't bought anything yet." />
    </div>
  );
}
