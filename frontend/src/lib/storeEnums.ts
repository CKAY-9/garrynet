import type { FulfillmentStatus, ItemStore, ReceiptStatus, Reward, RewardType, Server } from "./types";

/*
 * Store enums (docs/STORE.md) with their labels, plus money formatting. Safe to import from
 * client components.
 */

export const receiptStatuses: ReceiptStatus[] = ["PAID", "REFUNDED", "DISPUTED"];

export const receiptStatusLabels: Record<ReceiptStatus, string> = {
  PAID: "Paid",
  REFUNDED: "Refunded",
  DISPUTED: "Disputed",
};

export const fulfillmentStatusLabels: Record<FulfillmentStatus, string> = {
  PENDING: "Awaiting delivery",
  FULFILLED: "Delivered",
  FAILED: "Delivery failed",
  REVOKED: "Revoked",
};

export const rewardTypes: RewardType[] = ["usergroup", "darkrp_money", "weapon"];

export const rewardTypeLabels: Record<RewardType, string> = {
  usergroup: "Usergroup",
  darkrp_money: "DarkRP money",
  weapon: "Weapon",
};

/** A store is named after its server unless it has an override. */
export function storeName(store: Pick<ItemStore, "override_name">, server: Pick<Server, "name">): string {
  return store.override_name ?? server.name;
}

/** How many decimal digits the currency's minor unit has (2 for USD, 0 for JPY). */
export function minorUnitDigits(currency: string): number {
  try {
    const digits = new Intl.NumberFormat("en", { style: "currency", currency: currency.toUpperCase() }).resolvedOptions()
      .maximumFractionDigits;
    return digits ?? 2;
  } catch {
    return 2;
  }
}

/** Minor units as a currency string: `formatMoney(1999, "usd")` is `$19.99`. */
export function formatMoney(minor: number, currency: string): string {
  const digits = minorUnitDigits(currency);
  try {
    return new Intl.NumberFormat("en", { style: "currency", currency: currency.toUpperCase() }).format(
      minor / 10 ** digits,
    );
  } catch {
    return `${(minor / 10 ** digits).toFixed(digits)} ${currency.toUpperCase()}`;
  }
}

/** A seconds duration as "30 days", "12 hours" or "45 minutes". */
export function formatDuration(seconds: number): string {
  const units: [string, number][] = [
    ["day", 86_400],
    ["hour", 3600],
    ["minute", 60],
  ];
  for (const [unit, size] of units) {
    if (seconds >= size) {
      const n = Math.round(seconds / size);
      return `${n} ${unit}${n === 1 ? "" : "s"}`;
    }
  }
  return `${seconds} second${seconds === 1 ? "" : "s"}`;
}

/** A reward in words, e.g. "VIP usergroup for 30 days". */
export function describeReward(reward: Reward): string {
  switch (reward.type) {
    case "usergroup":
      return reward.duration_seconds
        ? `${reward.usergroup} usergroup for ${formatDuration(reward.duration_seconds)}`
        : `${reward.usergroup} usergroup (permanent)`;
    case "darkrp_money":
      return `$${reward.amount.toLocaleString("en")} DarkRP money`;
    case "weapon":
      return `Weapon: ${reward.class}`;
  }
}

/** Limits enforced by the backend (src/routes/store_admin.rs). */
export const MAX_NAME = 128;
export const MAX_MESSAGE = 2000;
export const MAX_SHORT_DESCRIPTION = 256;
export const MAX_FULL_DESCRIPTION = 20_000;
export const MAX_REWARDS = 20;
export const MAX_USERGROUP = 64;
export const MAX_WEAPON_CLASS = 128;
