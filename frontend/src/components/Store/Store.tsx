import Link from "next/link";
import type { CSSProperties } from "react";
import { Avatar } from "@/components/Avatar";
import { Markdown } from "@/components/Markdown";
import {
  describeReward,
  formatMoney,
  fulfillmentStatusLabels,
  receiptStatusLabels,
  storeName,
} from "@/lib/storeEnums";
import { fullFormat, timeAgo } from "@/lib/time";
import type {
  FulfillmentStatus,
  PricedItem,
  ReceiptStatus,
  ReceiptView,
  Reward,
  Server,
  StoreDiscount,
  StoreSummary,
  StoreWithServer,
} from "@/lib/types";
import styles from "./Store.module.scss";

type Crumb = { href?: string; label: string };

export function Breadcrumbs({ items }: { items: Crumb[] }) {
  return (
    <nav aria-label="Breadcrumb">
      <ol className={styles.crumbs}>
        {items.map((c, i) => (
          <li key={i}>{c.href ? <Link href={c.href}>{c.label}</Link> : c.label}</li>
        ))}
      </ol>
    </nav>
  );
}

/** "3 hours ago", with the full date on hover. */
export function When({ iso }: { iso: string }) {
  const date = new Date(iso);
  return (
    <time dateTime={iso} title={fullFormat.format(date)}>
      {timeAgo(date)}
    </time>
  );
}

/** Tabs between the store list and the user's purchases. */
export function StoreNav({ loggedIn, active }: { loggedIn: boolean; active: "stores" | "purchases" }) {
  const tabs = [
    { key: "stores", href: "/store", label: "Stores" },
    ...(loggedIn ? [{ key: "purchases", href: "/store/purchases", label: "My purchases" }] : []),
  ];
  return (
    <nav aria-label="Store">
      <ul className={styles.tabs}>
        {tabs.map((tab) => (
          <li key={tab.key}>
            <Link
              className={`${styles.tab} ${tab.key === active ? styles.activeTab : ""}`}
              href={tab.href}
              aria-current={tab.key === active ? "page" : undefined}
            >
              {tab.label}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}

/** "-25%" pill. */
export function DiscountBadge({ percent }: { percent: number }) {
  if (percent <= 0) return null;
  return <span className={`${styles.badge} ${styles.discount}`}>-{percent}%</span>;
}

/** The final price, with the list price struck through when discounted. */
export function Price({ item, currency, large = false }: { item: PricedItem; currency: string; large?: boolean }) {
  return (
    <span className={`${styles.price} ${large ? styles.priceLarge : ""}`}>
      <span className={styles.finalPrice}>{formatMoney(item.final_price, currency)}</span>
      {item.total_discount > 0 && (
        <>
          <s className={styles.listPrice}>{formatMoney(item.price, currency)}</s>
          <DiscountBadge percent={item.total_discount} />
        </>
      )}
    </span>
  );
}

/** Store cards for `/store`, each tinted by its server's color. */
export function StoreGrid({ stores, empty }: { stores: StoreWithServer[]; empty: string }) {
  if (stores.length === 0) return <p className={styles.notice}>{empty}</p>;
  return (
    <ul className={styles.grid}>
      {stores.map((store) => (
        <li key={store.id}>
          <Link
            href={`/store/${store.id}`}
            className={styles.storeCard}
            style={{ "--server-color": store.server.color } as CSSProperties}
          >
            <span className={styles.storeHead}>
              <Avatar name={storeName(store, store.server)} url={store.server.icon_url} size={48} />
              <span className={styles.storeIdentity}>
                <span className={styles.storeTitle}>{storeName(store, store.server)}</span>
                <span className={styles.meta}>
                  {[store.server.gamemode, store.server.map].filter(Boolean).join(" · ") || store.server.name}
                </span>
              </span>
            </span>
            <span className={styles.storeFoot}>
              <span>
                {store.item_count.toLocaleString("en")} {store.item_count === 1 ? "item" : "items"}
              </span>
              {store.store_discount > 0 && (
                <span className={styles.saleTag}>
                  <DiscountBadge percent={store.store_discount} /> store-wide
                </span>
              )}
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}

/** A store page's header: its name, server and message, plus active store-wide discounts. */
export function StoreHeader({
  store,
  server,
  discounts,
}: {
  store: StoreSummary;
  server: Server;
  discounts: StoreDiscount[];
}) {
  const name = storeName(store, server);
  return (
    <header className={`${styles.storeHero} ${server.splash_url ? styles.splash : ""}`}
      style={
        {
          "--server-color": server.color,
          ...(server.splash_url && { backgroundImage: `url(${JSON.stringify(server.splash_url)})` }),
        } as CSSProperties
      }
    >
      <div className={styles.heroRow}>
        <Avatar name={name} url={server.icon_url} size={64} />
        <div className={styles.heroText}>
          <h1>{name}</h1>
          <p className={styles.meta}>
            {store.override_name ? `${server.name} · ` : ""}
            {[server.gamemode, server.map].filter(Boolean).join(" · ") || "Garry's Mod server"}
          </p>
        </div>
      </div>
      {store.store_message && <p className={styles.storeMessage}>{store.store_message}</p>}
      {discounts.length > 0 && (
        <ul className={styles.sales} aria-label="Store-wide discounts">
          {discounts.map((d) => (
            <li key={d.id} className={styles.sale}>
              <DiscountBadge percent={d.discount} />
              <span>{d.message || "Store-wide discount"}</span>
              {d.ends_at && (
                <span className={styles.meta}>
                  ends <When iso={d.ends_at} />
                </span>
              )}
            </li>
          ))}
        </ul>
      )}
    </header>
  );
}

/** Stock and availability as a short tag, or nothing for an ordinary item. */
export function StockBadge({ item }: { item: PricedItem }) {
  if (!item.available) return <span className={`${styles.badge} ${styles.muted}`}>Unavailable</span>;
  if (item.remaining_stock === 0) return <span className={`${styles.badge} ${styles.soldOut}`}>Sold out</span>;
  if (item.remaining_stock !== null) {
    return (
      <span className={`${styles.badge} ${styles.limited}`}>
        {item.remaining_stock.toLocaleString("en")} left
      </span>
    );
  }
  return null;
}

/** Item cards for a store page, linking to each item. */
export function ItemGrid({ storeId, items, currency, empty }: { storeId: number; items: PricedItem[]; currency: string; empty: string }) {
  if (items.length === 0) return <p className={styles.notice}>{empty}</p>;
  return (
    <ul className={styles.grid}>
      {items.map((item) => (
        <li key={item.id}>
          <Link href={`/store/${storeId}/items/${item.id}`} className={`${styles.itemCard} ${item.available ? "" : styles.dim}`}>
            <ItemImage item={item} />
            <span className={styles.itemBody}>
              <span className={styles.itemTitleRow}>
                <span className={styles.itemTitle}>{item.name}</span>
                <StockBadge item={item} />
              </span>
              {item.short_description && <span className={styles.itemBlurb}>{item.short_description}</span>}
              <Price item={item} currency={currency} />
            </span>
          </Link>
        </li>
      ))}
    </ul>
  );
}

/** The item's picture, or a tile with its initial. */
export function ItemImage({ item, large = false }: { item: PricedItem; large?: boolean }) {
  const className = `${styles.itemImage} ${large ? styles.itemImageLarge : ""}`;
  if (item.image_url) {
    // Store images come from arbitrary hosts, so next/image's optimizer can't be used.
    // eslint-disable-next-line @next/next/no-img-element
    return <img className={className} src={item.image_url} alt="" loading="lazy" />;
  }
  return (
    <span className={`${className} ${styles.itemInitial}`} aria-hidden>
      {item.name.trim().charAt(0).toUpperCase() || "?"}
    </span>
  );
}

/** What an item grants, one line per reward. */
export function RewardList({ rewards }: { rewards: Reward[] }) {
  if (rewards.length === 0) return <p className={styles.hint}>No rewards listed.</p>;
  return (
    <ul className={styles.rewards}>
      {rewards.map((r, i) => (
        <li key={i} className={styles.reward}>
          <RewardIcon type={r.type} />
          {describeReward(r)}
        </li>
      ))}
    </ul>
  );
}

function RewardIcon({ type }: { type: Reward["type"] }) {
  const common = { viewBox: "0 0 24 24", width: 18, height: 18, fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round" as const, strokeLinejoin: "round" as const, "aria-hidden": true };
  switch (type) {
    case "usergroup":
      return (
        <svg {...common}>
          <path d="M12 2l3 7h7l-5.5 4.5L18 21l-6-4-6 4 1.5-7.5L2 9h7z" />
        </svg>
      );
    case "darkrp_money":
      return (
        <svg {...common}>
          <rect x="2" y="6" width="20" height="12" rx="2" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      );
    case "weapon":
      return (
        <svg {...common}>
          <path d="M3 21l6-6" />
          <path d="M14 4l6 6-9 9-6-6z" />
          <path d="M17 7l3-3" />
        </svg>
      );
  }
}

/** The item page's main content (the buy button is passed in, being client-side). */
export function ItemDetails({ item, currency, children }: { item: PricedItem; currency: string; children: React.ReactNode }) {
  return (
    <div className={styles.itemLayout}>
      <div className={styles.itemMain}>
        <ItemImage item={item} large />
        {item.full_description ? (
          <article className={styles.article}>
            <Markdown>{item.full_description}</Markdown>
          </article>
        ) : (
          item.short_description && <p className={styles.itemLead}>{item.short_description}</p>
        )}
      </div>
      <aside className={styles.itemAside} aria-label="Purchase">
        <Price item={item} currency={currency} large />
        <StockBadge item={item} />
        {children}
        <section>
          <h2>Rewards</h2>
          <RewardList rewards={item.rewards} />
        </section>
        <dl className={styles.facts}>
          {item.max_per_user !== null && (
            <div>
              <dt>Limit</dt>
              <dd>
                {item.max_per_user} per player
              </dd>
            </div>
          )}
          {item.starting_stock !== null && (
            <div>
              <dt>Stock</dt>
              <dd>
                {item.remaining_stock?.toLocaleString("en") ?? 0} of {item.starting_stock.toLocaleString("en")} left
              </dd>
            </div>
          )}
          <div>
            <dt>Sold</dt>
            <dd>{item.all_time_purchases.toLocaleString("en")}</dd>
          </div>
        </dl>
      </aside>
    </div>
  );
}

export function ReceiptStatusBadge({ status }: { status: ReceiptStatus }) {
  return <span className={`${styles.badge} ${styles[`receipt_${status.toLowerCase()}`]}`}>{receiptStatusLabels[status]}</span>;
}

export function FulfillmentBadge({ status }: { status: FulfillmentStatus | null }) {
  if (!status) return <span className={`${styles.badge} ${styles.muted}`}>No delivery</span>;
  return <span className={`${styles.badge} ${styles[`delivery_${status.toLowerCase()}`]}`}>{fulfillmentStatusLabels[status]}</span>;
}

/** A buyer's receipts, with how delivery is going. */
export function PurchaseRows({ receipts, empty }: { receipts: ReceiptView[]; empty: string }) {
  return (
    <ul className={styles.rows}>
      {receipts.map((r) => (
        <li key={r.id} className={styles.row}>
          <span className={styles.rowMain}>
            <span className={styles.rowTitle}>
              {r.item_id !== null && r.store_id !== null ? (
                <Link href={`/store/${r.store_id}/items/${r.item_id}`}>{r.item_name}</Link>
              ) : (
                r.item_name
              )}
              <ReceiptStatusBadge status={r.status} />
              {r.status === "PAID" && <FulfillmentBadge status={r.fulfillment?.status ?? null} />}
            </span>
            <span className={styles.meta}>
              {r.store_name} · bought <When iso={r.purchased_at} /> · receipt #{r.id}
            </span>
            {r.fulfillment && <DeliveryNote receipt={r} />}
          </span>
          <span className={styles.rowPrice}>
            <strong>{formatMoney(r.price, r.currency)}</strong>
            {r.discount_percentage > 0 && (
              <span className={styles.meta}>
                <s>{formatMoney(r.list_price, r.currency)}</s> · {r.discount_percentage}% off
              </span>
            )}
          </span>
        </li>
      ))}
      {receipts.length === 0 && <li className={styles.empty}>{empty}</li>}
    </ul>
  );
}

function DeliveryNote({ receipt }: { receipt: ReceiptView }) {
  const f = receipt.fulfillment;
  if (!f) return null;
  switch (f.status) {
    case "PENDING":
      return <span className={styles.meta}>Rewards are delivered the next time you join the server.</span>;
    case "FULFILLED":
      return (
        <span className={styles.meta}>
          Delivered {f.fulfilled_at && <When iso={f.fulfilled_at} />}
        </span>
      );
    case "FAILED":
      return (
        <span className={styles.meta}>
          Delivery failed {f.attempts} {f.attempts === 1 ? "time" : "times"}; it will be retried. If this keeps happening, open a support ticket.
        </span>
      );
    case "REVOKED":
      return (
        <span className={styles.meta}>
          {receipt.status === "REFUNDED" ? "Refunded" : "Disputed"}; usergroups and weapons from this purchase are removed.
        </span>
      );
  }
}
