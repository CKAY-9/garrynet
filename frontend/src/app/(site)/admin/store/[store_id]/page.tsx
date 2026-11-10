import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { requirePermission } from "@/lib/auth";
import { hasPermission, Permission } from "@/lib/permissions";
import { getAdminStore } from "@/lib/store";
import { describeReward, formatMoney, storeName } from "@/lib/storeEnums";
import type { PricedItem } from "@/lib/types";
import styles from "../../admin.module.scss";
import form from "../../community/editor.module.scss";
import list from "../store.module.scss";
import { DiscountForm, DiscountRow } from "./Discounts";
import { GrantableUsergroups } from "./GrantableUsergroups";
import { StoreSettings } from "./StoreSettings";

export default function StoreSettingsPage({ params }: PageProps<"/admin/store/[store_id]">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      {params.then(({ store_id }) => (
        <ManageStore id={store_id} />
      ))}
    </Suspense>
  );
}

/** /admin/store/{id}: settings, grantable usergroups, items and discounts of one store. */
async function ManageStore({ id }: { id: string }) {
  const user = await requirePermission(Permission.AdminAccess | Permission.StoreManagement);
  const elevated = hasPermission(user.permissions, Permission.ElevatedControl);
  const page = await getAdminStore(id);
  if (!page) notFound();
  const name = storeName(page, page.server);

  return (
    <>
      <Link href="/admin/store" className={list.back}>
        ← All stores
      </Link>

      <section className={styles.section} aria-labelledby="store-heading">
        <div className={list.titleRow}>
          <h2 id="store-heading" className={styles.sectionTitle}>
            {name} <span className={list.id}>#{page.id}</span>
            <span className={`${list.badge} ${page.enabled ? list.on : list.off}`}>{page.enabled ? "Enabled" : "Disabled"}</span>
          </h2>
          <div className={list.headActions}>
            <Link href={`/store/${page.id}`} className={list.secondaryLink}>
              View as a player
            </Link>
            <Link href={`/admin/servers/${page.server.id}`} className={list.secondaryLink}>
              Server settings
            </Link>
          </div>
        </div>
        <StoreSettings store={page} hasItems={page.items.length > 0} />
      </section>

      <section className={styles.section} aria-labelledby="grantable-heading">
        <h2 id="grantable-heading" className={styles.sectionTitle}>
          Grantable usergroups
        </h2>
        <GrantableUsergroups storeId={page.id} groups={page.grantable_usergroups} canEdit={elevated} />
      </section>

      <section className={styles.section} aria-labelledby="items-heading">
        <div className={list.titleRow}>
          <h2 id="items-heading" className={styles.sectionTitle}>
            Items
          </h2>
          <Link href={`/admin/store/${page.id}/items/new`} className={`${form.save} ${list.linkButton}`}>
            New item
          </Link>
        </div>
        {page.items.length === 0 ? (
          <p className={styles.placeholder}>No items yet. Players see an empty store until you add one.</p>
        ) : (
          <ul className={list.list}>
            {page.items.map((item) => (
              <ItemRow key={item.id} storeId={page.id} item={item} currency={page.currency} />
            ))}
          </ul>
        )}
      </section>

      <section className={styles.section} aria-labelledby="discounts-heading">
        <h2 id="discounts-heading" className={styles.sectionTitle}>
          Store-wide discounts
        </h2>
        <p className={styles.note}>
          Active discounts add to each item&apos;s own discount, capped at 100% in total.
          {page.store_discount > 0 && ` Right now ${page.store_discount}% is applied store-wide.`}
        </p>
        {page.discounts.length > 0 && (
          <ul className={list.list}>
            {page.discounts.map((d) => (
              <DiscountRow key={d.id} storeId={page.id} discount={d} />
            ))}
          </ul>
        )}
        <DiscountForm storeId={page.id} />
      </section>
    </>
  );
}

function ItemRow({ storeId, item, currency }: { storeId: number; item: PricedItem; currency: string }) {
  const stock =
    item.starting_stock === null ? "unlimited stock" : `${item.remaining_stock ?? 0} of ${item.starting_stock} left`;
  return (
    <li>
      <Link href={`/admin/store/${storeId}/items/${item.id}`} className={`${list.card} ${item.available ? "" : list.dim}`}>
        {item.image_url ? (
          // Store images come from arbitrary hosts, so next/image's optimizer can't be used.
          // eslint-disable-next-line @next/next/no-img-element
          <img className={list.thumb} src={item.image_url} alt="" loading="lazy" />
        ) : (
          <span className={`${list.thumb} ${list.thumbInitial}`} aria-hidden>
            {item.name.trim().charAt(0).toUpperCase() || "?"}
          </span>
        )}
        <span className={list.cardBody}>
          <span className={list.cardTitle}>
            {item.name}
            <span className={list.id}>#{item.id}</span>
            {!item.available && <span className={`${list.badge} ${list.off}`}>Unavailable</span>}
            {item.remaining_stock === 0 && <span className={`${list.badge} ${list.warn}`}>Sold out</span>}
          </span>
          <span className={list.cardMeta}>
            {item.rewards.map(describeReward).join(" · ") || "No rewards"}
          </span>
          <span className={list.cardMeta}>
            {stock}
            {item.max_per_user !== null && ` · ${item.max_per_user} per player`} · {item.all_time_purchases} sold · order {item.sort_order}
          </span>
        </span>
        <span className={list.cardEnd}>
          <strong>{formatMoney(item.final_price, currency)}</strong>
          {item.total_discount > 0 && (
            <span className={list.cardMeta}>
              <s>{formatMoney(item.price, currency)}</s> · {item.total_discount}% off
            </span>
          )}
        </span>
      </Link>
    </li>
  );
}
