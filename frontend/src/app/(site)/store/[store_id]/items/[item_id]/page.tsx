import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Breadcrumbs, BuyButton, ItemDetails } from "@/components/Store";
import { firstParam, getItem, getStoreViewer, type SearchParams } from "@/lib/store";
import { storeName } from "@/lib/storeEnums";
import styles from "@/components/Store/Store.module.scss";

export default function StoreItemPage({ params, searchParams }: PageProps<"/store/[store_id]/items/[item_id]">) {
  return (
    // Params, the item and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {params.then(({ store_id, item_id }) => (
        <Item storeId={store_id} itemId={item_id} searchParams={searchParams} />
      ))}
    </Suspense>
  );
}

/** /store/{id}/items/{id}: an item's description, rewards and the buy button. Public. */
async function Item({ storeId, itemId, searchParams }: { storeId: string; itemId: string; searchParams: SearchParams }) {
  const [viewer, page, params] = await Promise.all([getStoreViewer(), getItem(storeId, itemId), searchParams]);
  if (!page) notFound();
  const { store, server, item, buyer } = page;
  const name = storeName(store, server);
  return (
    <div className={styles.page}>
      <div className={styles.sectionHead}>
        <Breadcrumbs items={[{ href: "/store", label: "Store" }, { href: `/store/${store.id}`, label: name }, { label: item.name }]} />
        {viewer.manage && <Link href={`/admin/store/${store.id}/items/${item.id}`}>Edit this item</Link>}
      </div>
      <header className={styles.header}>
        <h1>{item.name}</h1>
        {item.short_description && <p>{item.short_description}</p>}
      </header>
      <ItemDetails item={item} currency={store.currency}>
        <BuyButton item={item} currency={store.currency} eligibility={buyer} cancelled={firstParam(params, "cancelled") === "1"} />
      </ItemDetails>
    </div>
  );
}
