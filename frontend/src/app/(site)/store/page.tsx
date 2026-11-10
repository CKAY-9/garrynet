import { Suspense } from "react";
import Link from "next/link";
import { StoreGrid, StoreNav } from "@/components/Store";
import { getStores, getStoreViewer } from "@/lib/store";
import styles from "@/components/Store/Store.module.scss";

export default function StorePage() {
  return (
    // The session and stores are request-time, so they stream in behind the boundary.
    <Suspense>
      <Stores />
    </Suspense>
  );
}

/** /store: every server with an enabled item store. */
async function Stores() {
  const [viewer, stores] = await Promise.all([getStoreViewer(), getStores()]);
  return (
    <div className={styles.page}>
      <div className={styles.sectionHead}>
        <header className={styles.header}>
          <h1>Store</h1>
          <p>Support the community and get in-game rewards on its servers.</p>
        </header>
        {viewer.manage && <Link href="/admin/store">Manage stores</Link>}
      </div>
      <StoreNav loggedIn={viewer.userId !== null} active="stores" />
      <StoreGrid stores={stores} empty="No server has a store yet." />
      {!viewer.userId && (
        <p className={styles.hint}>You can browse without an account; log in with Steam when you want to buy.</p>
      )}
    </div>
  );
}
