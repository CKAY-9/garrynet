import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import { getAdminStore } from "@/lib/store";
import { storeName } from "@/lib/storeEnums";
import styles from "../../../../admin.module.scss";
import list from "../../../store.module.scss";
import { ItemForm } from "../ItemForm";

export default function NewItemPage({ params }: PageProps<"/admin/store/[store_id]/items/new">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      {params.then(({ store_id }) => (
        <NewItem storeId={store_id} />
      ))}
    </Suspense>
  );
}

/** /admin/store/{id}/items/new: adds an item to the store. */
async function NewItem({ storeId }: { storeId: string }) {
  await requirePermission(Permission.AdminAccess | Permission.StoreManagement);
  const page = await getAdminStore(storeId);
  if (!page) notFound();
  return (
    <>
      <Link href={`/admin/store/${page.id}`} className={list.back}>
        ← {storeName(page, page.server)}
      </Link>
      <section className={styles.section} aria-labelledby="new-item-heading">
        <h2 id="new-item-heading" className={styles.sectionTitle}>
          New item
        </h2>
        <ItemForm store={page} server={page.server} />
      </section>
    </>
  );
}
