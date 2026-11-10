"use client";

import { useState, useTransition } from "react";
import { steamLoginUrl } from "@/lib/config";
import { formatMoney } from "@/lib/storeEnums";
import type { Eligibility, PricedItem } from "@/lib/types";
import { startCheckout } from "./actions";
import styles from "./Store.module.scss";

/**
 * The buy button on an item page. Logged-out users are sent to Steam; everyone else gets
 * the backend's verdict (`eligibility`), and a click opens Stripe Checkout in this tab.
 */
export function BuyButton({
  item,
  currency,
  eligibility,
  cancelled,
}: {
  item: PricedItem;
  currency: string;
  /** `null` when logged out. */
  eligibility: Eligibility | null;
  /** The buyer just backed out of Checkout. */
  cancelled: boolean;
}) {
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const price = formatMoney(item.final_price, currency);

  if (!eligibility) {
    return (
      <div className={styles.buy}>
        {/* Plain anchor: a full navigation to the backend, which redirects to Steam. */}
        <a className={styles.primary} href={steamLoginUrl}>
          Log in with Steam to buy
        </a>
        <p className={styles.hint}>Rewards are tied to the Steam account you log in with.</p>
      </div>
    );
  }

  function onBuy() {
    startTransition(async () => {
      const res = await startCheckout(item.store_id, item.id);
      if (res.ok) {
        // A full navigation: Stripe hosts the payment page.
        window.location.assign(res.data.url);
      } else {
        setError(res.message);
      }
    });
  }

  return (
    <div className={styles.buy}>
      <button type="button" className={styles.primary} onClick={onBuy} disabled={pending || !eligibility.can_buy}>
        {pending ? "Opening checkout..." : `Buy for ${price}`}
      </button>
      {!eligibility.can_buy && eligibility.reason && <p className={styles.hint}>{capitalize(eligibility.reason)}.</p>}
      {eligibility.can_buy && cancelled && !error && (
        <p className={styles.hint} role="status">
          Checkout was cancelled. Nothing was charged.
        </p>
      )}
      {eligibility.can_buy && !cancelled && !error && (
        <p className={styles.hint}>You&apos;ll pay on Stripe&apos;s secure checkout page and come straight back.</p>
      )}
      {error && (
        <p className={styles.error} role="status">
          {error}
        </p>
      )}
    </div>
  );
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
