import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import { getAdminStore } from "@/lib/store";
import { formatMoney, storeName } from "@/lib/storeEnums";
import { dateFormat, fullFormat } from "@/lib/time";
import styles from "../../../../admin.module.scss";
import list from "../../../store.module.scss";
import { ArchiveItem, ItemForm } from "../ItemForm";

export default function EditItemPage({ params }: PageProps<"/admin/store/[store_id]/items/[item_id]">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      {params.then(({ store_id, item_id }) => (
        <EditItem storeId={store_id} itemId={item_id} />
      ))}
    </Suspense>
  );
}

/** /admin/store/{id}/items/{id}: edits or archives an item. */
async function EditItem({ storeId, itemId }: { storeId: string; itemId: string }) {
  await requirePermission(Permission.AdminAccess | Permission.StoreManagement);
  const page = await getAdminStore(storeId);
  const item = page?.items.find((i) => String(i.id) === itemId);
  if (!page || !item) notFound();
  return (
    <>
      <Link href={`/admin/store/${page.id}`} className={list.back}>
        ← {storeName(page, page.server)}
      </Link>
      <section className={styles.section} aria-labelledby="item-heading">
        <div className={list.titleRow}>
          <h2 id="item-heading" className={styles.sectionTitle}>
            {item.name} <span className={list.id}>#{item.id}</span>
          </h2>
          <div className={list.headActions}>
            <Link href={`/store/${page.id}/items/${item.id}`} className={list.secondaryLink}>
              View as a player
            </Link>
          </div>
        </div>
        <dl className={list.grid}>
          <div>
            <dt className={list.cardMeta}>Current price</dt>
            <dd style={{ margin: 0 }}>
              {formatMoney(item.final_price, page.currency)}
              {item.total_discount > 0 && <span className={list.cardMeta}> after {item.total_discount}% off</span>}
            </dd>
          </div>
          <div>
            <dt className={list.cardMeta}>Sold</dt>
            <dd style={{ margin: 0 }}>{item.all_time_purchases.toLocaleString("en")}</dd>
          </div>
          <div>
            <dt className={list.cardMeta}>Created</dt>
            <dd style={{ margin: 0 }}>
              <time dateTime={item.created_at} title={fullFormat.format(new Date(item.created_at))}>
                {dateFormat.format(new Date(item.created_at))}
              </time>
            </dd>
          </div>
        </dl>
        {/* Keyed so a save (which refreshes the page) resets the form to the saved item. */}
        <ItemForm key={item.last_updated ?? item.created_at} store={page} server={page.server} item={item} />
      </section>
      <section className={styles.section} aria-labelledby="archive-heading">
        <h2 id="archive-heading" className={styles.sectionTitle}>
          Archive
        </h2>
        <ArchiveItem storeId={page.id} item={item} />
      </section>
    </>
  );
}
