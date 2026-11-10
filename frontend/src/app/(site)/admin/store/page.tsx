import { Suspense } from "react";
import Link from "next/link";
import { forbidden } from "next/navigation";
import { Avatar } from "@/components/Avatar";
import { requirePermission } from "@/lib/auth";
import { hasPermission, Permission } from "@/lib/permissions";
import { getServers } from "@/lib/servers";
import { getAdminStores } from "@/lib/store";
import { storeName } from "@/lib/storeEnums";
import styles from "../admin.module.scss";
import { CreateStoreForm } from "./CreateStoreForm";
import list from "./store.module.scss";

export default function StoreAdminPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <StoreAdmin />
    </Suspense>
  );
}

/** /admin/store: every store, and a form to add one. Store Management or Store Finance. */
async function StoreAdmin() {
  const user = await requirePermission(Permission.AdminAccess);
  const manage = hasPermission(user.permissions, Permission.StoreManagement);
  const finance = hasPermission(user.permissions, Permission.StoreFinance);
  if (!manage && !finance) forbidden();

  const [stores, servers] = await Promise.all([getAdminStores(), getServers()]);
  const withoutStore = servers.filter((s) => !stores.some((store) => store.server_id === s.id));

  return (
    <>
      <section className={styles.section} aria-labelledby="stores-heading">
        <div className={list.titleRow}>
          <h2 id="stores-heading" className={styles.sectionTitle}>
            Item stores
          </h2>
          <div className={list.headActions}>
            {finance && (
              <Link href="/admin/store/receipts" className={list.secondaryLink}>
                Receipts and refunds
              </Link>
            )}
            <Link href="/store" className={list.secondaryLink}>
              View the store
            </Link>
          </div>
        </div>
        {stores.length === 0 ? (
          <p className={styles.placeholder}>No stores yet. {manage ? "Create one below." : ""}</p>
        ) : (
          <ul className={list.list}>
            {stores.map((store) => (
              <li key={store.id}>
                <Link
                  href={manage ? `/admin/store/${store.id}` : `/store/${store.id}`}
                  className={list.card}
                  style={{ borderLeftColor: store.server.color }}
                >
                  <Avatar name={storeName(store, store.server)} url={store.server.icon_url} size={44} />
                  <span className={list.cardBody}>
                    <span className={list.cardTitle}>
                      {storeName(store, store.server)}
                      <span className={list.id}>#{store.id}</span>
                      <span className={`${list.badge} ${store.enabled ? list.on : list.off}`}>
                        {store.enabled ? "Enabled" : "Disabled"}
                      </span>
                    </span>
                    <span className={list.cardMeta}>
                      {store.override_name ? `${store.server.name} · ` : ""}
                      {store.currency.toUpperCase()}
                      {store.grantable_usergroups.length > 0 && ` · grants ${store.grantable_usergroups.join(", ")}`}
                    </span>
                  </span>
                  <span className={list.cardEnd}>
                    <span>
                      {store.item_count} {store.item_count === 1 ? "item" : "items"}
                    </span>
                    {store.store_discount > 0 && <span className={list.cardMeta}>{store.store_discount}% off store-wide</span>}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>

      {manage && (
        <section className={styles.section} aria-labelledby="create-heading">
          <h2 id="create-heading" className={styles.sectionTitle}>
            Create a store
          </h2>
          <p className={styles.note}>
            Each server can have one store. GarryNet prohibits gambling, loot boxes and other luck-based items; item
            names and descriptions must not imply random rewards.
          </p>
          <CreateStoreForm servers={withoutStore.map(({ id, name }) => ({ id, name }))} />
        </section>
      )}
    </>
  );
}
