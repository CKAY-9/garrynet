import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Breadcrumbs, ItemGrid, StoreHeader } from "@/components/Store";
import { getStore, getStoreViewer } from "@/lib/store";
import { storeName } from "@/lib/storeEnums";
import styles from "@/components/Store/Store.module.scss";

export default function ItemStorePage({ params }: PageProps<"/store/[store_id]">) {
  return (
    // Params, the store and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {params.then(({ store_id }) => (
        <ItemStore id={store_id} />
      ))}
    </Suspense>
  );
}

/** /store/{id}: a server's store and its items. Public. */
async function ItemStore({ id }: { id: string }) {
  const [viewer, page] = await Promise.all([getStoreViewer(), getStore(id)]);
  if (!page) notFound();
  const name = storeName(page, page.server);
  return (
    <div className={styles.page}>
      <div className={styles.sectionHead}>
        <Breadcrumbs items={[{ href: "/store", label: "Store" }, { label: name }]} />
        {viewer.manage && <Link href={`/admin/store/${page.id}`}>Manage this store</Link>}
      </div>
      <StoreHeader store={page} server={page.server} discounts={page.discounts} />
      <section className={styles.section} aria-labelledby="items-heading">
        <div className={styles.sectionHead}>
          <h2 id="items-heading">Items</h2>
          <span>
            {page.items.length} {page.items.length === 1 ? "item" : "items"}
          </span>
        </div>
        <ItemGrid storeId={page.id} items={page.items} currency={page.currency} empty="Nothing for sale here yet." />
      </section>
    </div>
  );
}
