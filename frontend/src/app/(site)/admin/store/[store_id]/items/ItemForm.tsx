"use client";

import { useState, useTransition, type FormEvent } from "react";
import {
  MAX_FULL_DESCRIPTION,
  MAX_NAME,
  MAX_REWARDS,
  MAX_SHORT_DESCRIPTION,
  MAX_WEAPON_CLASS,
  minorUnitDigits,
  rewardTypeLabels,
  rewardTypes,
} from "@/lib/storeEnums";
import type { ItemDetails, ItemStore, Reward, RewardType, Server, StoreItem } from "@/lib/types";
import form from "../../../community/editor.module.scss";
import { Field } from "../../CreateStoreForm";
import { archiveItem, createItem, updateItem } from "../../actions";
import styles from "../../store.module.scss";

/** Form state: every field is a string while editing. */
type Draft = {
  name: string;
  short_description: string;
  full_description: string;
  image_url: string;
  sort_order: string;
  available: boolean;
  /** Major units, e.g. "4.99". */
  price: string;
  discount: string;
  starting_stock: string;
  max_per_user: string;
  rewards: RewardDraft[];
};

type RewardDraft = {
  type: RewardType;
  usergroup: string;
  /** Days; blank is permanent. */
  duration_days: string;
  amount: string;
  class: string;
};

const SECONDS_PER_DAY = 86_400;

function emptyReward(type: RewardType, usergroups: string[]): RewardDraft {
  return { type, usergroup: usergroups[0] ?? "", duration_days: "", amount: "", class: "" };
}

function toRewardDraft(r: Reward, usergroups: string[]): RewardDraft {
  const d = emptyReward(r.type, usergroups);
  switch (r.type) {
    case "usergroup":
      return { ...d, usergroup: r.usergroup, duration_days: r.duration_seconds ? String(r.duration_seconds / SECONDS_PER_DAY) : "" };
    case "darkrp_money":
      return { ...d, amount: String(r.amount) };
    case "weapon":
      return { ...d, class: r.class };
  }
}

function toReward(d: RewardDraft): Reward {
  switch (d.type) {
    case "usergroup": {
      const days = Number(d.duration_days);
      return {
        type: "usergroup",
        usergroup: d.usergroup.trim(),
        duration_seconds: days > 0 ? Math.round(days * SECONDS_PER_DAY) : null,
      };
    }
    case "darkrp_money":
      return { type: "darkrp_money", amount: Math.round(Number(d.amount)) || 0 };
    case "weapon":
      return { type: "weapon", class: d.class.trim() };
  }
}

function toDraft(item: StoreItem | undefined, digits: number, usergroups: string[]): Draft {
  return {
    name: item?.name ?? "",
    short_description: item?.short_description ?? "",
    full_description: item?.full_description ?? "",
    image_url: item?.image_url ?? "",
    sort_order: String(item?.sort_order ?? 0),
    available: item?.available ?? true,
    price: item ? (item.price / 10 ** digits).toFixed(digits) : "",
    discount: String(item?.discount ?? 0),
    starting_stock: item?.starting_stock === null || item === undefined ? "" : String(item.starting_stock),
    max_per_user: item?.max_per_user === null || item === undefined ? "" : String(item.max_per_user),
    rewards: item?.rewards.map((r) => toRewardDraft(r, usergroups)) ?? [emptyReward(usergroups.length > 0 ? "usergroup" : "darkrp_money", usergroups)],
  };
}

/** What gets sent to the backend: numbers parsed, blanks `null`, prices in minor units. */
function toDetails(d: Draft, digits: number): ItemDetails {
  const optionalInt = (v: string) => (v.trim() === "" ? null : Math.round(Number(v)));
  return {
    name: d.name.trim(),
    short_description: d.short_description.trim(),
    full_description: d.full_description.trim(),
    image_url: d.image_url.trim() || null,
    sort_order: Math.round(Number(d.sort_order)) || 0,
    available: d.available,
    price: Math.round((Number(d.price) || 0) * 10 ** digits),
    discount: Math.round(Number(d.discount)) || 0,
    starting_stock: optionalInt(d.starting_stock),
    max_per_user: optionalInt(d.max_per_user),
    rewards: d.rewards.map(toReward),
  };
}

/**
 * Creates an item in `store`, or edits `item` when given. The backend validates everything,
 * including rewards against the store's grantable usergroups and the server's gamemode.
 */
export function ItemForm({ store, server, item }: { store: ItemStore; server: Server; item?: StoreItem }) {
  const digits = minorUnitDigits(store.currency);
  const usergroups = store.grantable_usergroups;
  const [saved, setSaved] = useState(() => toDetails(toDraft(item, digits, usergroups), digits));
  const [draft, setDraft] = useState(() => toDraft(item, digits, usergroups));
  const [error, setError] = useState<string | null>(null);
  const [savedMessage, setSavedMessage] = useState(false);
  const [pending, startTransition] = useTransition();
  const details = toDetails(draft, digits);
  const changed = JSON.stringify(details) !== JSON.stringify(saved);
  const patch = (p: Partial<Draft>) => setDraft((d) => ({ ...d, ...p }));
  const patchReward = (i: number, p: Partial<RewardDraft>) =>
    setDraft((d) => ({ ...d, rewards: d.rewards.map((r, j) => (j === i ? { ...r, ...p } : r)) }));

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startTransition(async () => {
      if (item) {
        const res = await updateItem(store.id, item.id, details);
        if (res.ok) {
          const next = toDraft(res.data, digits, usergroups);
          setSaved(toDetails(next, digits));
          setDraft(next);
          setError(null);
          setSavedMessage(true);
        } else {
          setError(res.message);
        }
      } else {
        // Redirects to the store on success.
        const res = await createItem(store.id, details);
        if (!res.ok) setError(res.message);
      }
    });
  }

  const symbol = store.currency.toUpperCase();
  const isDarkRp = server.gamemode === "" || server.gamemode.toLowerCase().includes("darkrp");

  return (
    <form className={form.editor} onSubmit={onSubmit}>
      <fieldset className={form.group}>
        <legend className={form.legend}>Details</legend>
        <Field label="Name">
          <input className={form.input} value={draft.name} onChange={(e) => patch({ name: e.target.value })} maxLength={MAX_NAME} placeholder="VIP for a month" required />
        </Field>
        <Field label="Short description" hint="Plain text, shown on the item's card.">
          <input className={form.input} value={draft.short_description} onChange={(e) => patch({ short_description: e.target.value })} maxLength={MAX_SHORT_DESCRIPTION} />
        </Field>
        <Field label="Full description" hint="Markdown, shown on the item's page. Must not imply random or chance-based rewards.">
          <textarea className={`${form.input} ${form.textarea}`} value={draft.full_description} onChange={(e) => patch({ full_description: e.target.value })} maxLength={MAX_FULL_DESCRIPTION} rows={8} />
        </Field>
        <div className={styles.grid}>
          <Field label="Image URL" hint="Optional. Wide images work best.">
            <input className={form.input} type="url" value={draft.image_url} onChange={(e) => patch({ image_url: e.target.value })} placeholder="https://example.com/vip.png" />
          </Field>
          <Field label="Sort order" hint="Lower comes first.">
            <input className={form.input} type="number" step={1} value={draft.sort_order} onChange={(e) => patch({ sort_order: e.target.value })} required />
          </Field>
        </div>
      </fieldset>

      <fieldset className={form.group}>
        <legend className={form.legend}>Pricing and limits</legend>
        <div className={styles.grid}>
          <Field label={`Price (${symbol})`} hint="Before discounts. Stripe has a minimum charge (about 0.50 USD).">
            <input className={form.input} type="number" min={0} step={digits === 0 ? 1 : 1 / 10 ** digits} value={draft.price} onChange={(e) => patch({ price: e.target.value })} placeholder={digits === 0 ? "500" : "4.99"} required />
          </Field>
          <Field label="Discount (%)" hint="This item's own discount; store-wide discounts add to it.">
            <input className={form.input} type="number" min={0} max={100} step={1} value={draft.discount} onChange={(e) => patch({ discount: e.target.value })} required />
          </Field>
        </div>
        <div className={styles.grid}>
          <Field label="Stock" hint={item?.starting_stock != null ? "Changing it adjusts what's left by the same amount." : "Blank for unlimited. Limited items show how many are left."}>
            <input className={form.input} type="number" min={0} step={1} value={draft.starting_stock} onChange={(e) => patch({ starting_stock: e.target.value })} placeholder="Unlimited" />
          </Field>
          <Field label="Max per player" hint="Blank for no limit; 1 for one-time items.">
            <input className={form.input} type="number" min={1} step={1} value={draft.max_per_user} onChange={(e) => patch({ max_per_user: e.target.value })} placeholder="No limit" />
          </Field>
        </div>
        <label className={styles.checkbox}>
          <input type="checkbox" checked={draft.available} onChange={(e) => patch({ available: e.target.checked })} />
          <span>
            <span className={form.label}>Available</span>
            <span className={form.hint}>When off, the item is listed but can&apos;t be bought.</span>
          </span>
        </label>
      </fieldset>

      <fieldset className={form.group}>
        <legend className={form.legend}>Rewards</legend>
        <p className={styles.panelText}>What the buyer gets in-game. Delivered by the server&apos;s addon when they next join.</p>
        {draft.rewards.map((r, i) => (
          <div key={i} className={styles.rewardRow}>
            <Field label="Type">
              <select className={form.input} value={r.type} onChange={(e) => patchReward(i, { type: e.target.value as RewardType })}>
                {rewardTypes.map((t) => (
                  <option key={t} value={t}>
                    {rewardTypeLabels[t]}
                  </option>
                ))}
              </select>
            </Field>
            <div className={styles.rewardFields}>
              {r.type === "usergroup" && (
                <>
                  <Field label="Usergroup" hint={usergroups.length === 0 ? "This store can't grant any usergroup yet." : undefined}>
                    <select className={form.input} value={r.usergroup} onChange={(e) => patchReward(i, { usergroup: e.target.value })} disabled={usergroups.length === 0} required>
                      {usergroups.length === 0 && <option value="">None allowed</option>}
                      {usergroups.map((g) => (
                        <option key={g} value={g}>
                          {g}
                        </option>
                      ))}
                    </select>
                  </Field>
                  <Field label="Duration (days)" hint="Blank for permanent.">
                    <input className={form.input} type="number" min={0} step="any" value={r.duration_days} onChange={(e) => patchReward(i, { duration_days: e.target.value })} placeholder="Permanent" />
                  </Field>
                </>
              )}
              {r.type === "darkrp_money" && (
                <Field label="Amount" hint={isDarkRp ? undefined : `${server.name} doesn't run DarkRP; the backend will reject this.`}>
                  <input className={form.input} type="number" min={1} step={1} value={r.amount} onChange={(e) => patchReward(i, { amount: e.target.value })} required />
                </Field>
              )}
              {r.type === "weapon" && (
                <Field label="Weapon class" hint="e.g. weapon_crowbar. Permanent.">
                  <input className={form.input} value={r.class} onChange={(e) => patchReward(i, { class: e.target.value })} maxLength={MAX_WEAPON_CLASS} pattern="\S+" required />
                </Field>
              )}
            </div>
            <button
              type="button"
              className={form.iconButton}
              onClick={() => patch({ rewards: draft.rewards.filter((_, j) => j !== i) })}
              disabled={draft.rewards.length === 1}
              aria-label="Remove reward"
              title="Remove reward"
            >
              ×
            </button>
          </div>
        ))}
        <button
          type="button"
          className={form.add}
          onClick={() => patch({ rewards: [...draft.rewards, emptyReward(usergroups.length > 0 ? "usergroup" : "weapon", usergroups)] })}
          disabled={draft.rewards.length >= MAX_REWARDS}
        >
          + Add reward
        </button>
      </fieldset>

      <div className={form.actions}>
        <button type="submit" className={form.save} disabled={pending || (item !== undefined && !changed)}>
          {pending ? "Saving..." : item ? "Save" : "Create item"}
        </button>
        {item && (
          <button
            type="button"
            className={form.discard}
            onClick={() => {
              setDraft(toDraft({ ...item, ...saved, rewards: saved.rewards, id: item.id, store_id: item.store_id, created_at: item.created_at, last_updated: item.last_updated, archived: item.archived, remaining_stock: item.remaining_stock, all_time_purchases: item.all_time_purchases }, digits, usergroups));
              setError(null);
            }}
            disabled={pending || !changed}
          >
            Discard changes
          </button>
        )}
        <p className={form.message} role="status">
          {error && <span className={form.error}>{error}</span>}
          {!error && item && savedMessage && !changed && "Saved."}
          {!error && item && changed && !pending && "Unsaved changes."}
          {!error && !item && "Players can buy it as soon as it's created, if it's available."}
        </p>
      </div>
    </form>
  );
}

/** Archives the item after a second click. Archived items are hidden but kept for receipts. */
export function ArchiveItem({ storeId, item }: { storeId: number; item: StoreItem }) {
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onClick() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    startTransition(async () => {
      // Redirects to the store on success.
      const res = await archiveItem(storeId, item.id);
      if (!res.ok) {
        setError(res.message);
        setConfirming(false);
      }
    });
  }

  return (
    <div className={`${form.group} ${styles.dangerGroup}`}>
      <p className={styles.panelText}>
        Archiving hides <strong>{item.name}</strong> from players for good. Receipts and pending deliveries keep
        working. To pause sales instead, untick “Available” above.
      </p>
      <div className={styles.inlineActions}>
        <button type="button" className={confirming ? styles.dangerConfirm : styles.danger} onClick={onClick} onBlur={() => setConfirming(false)} disabled={pending}>
          {pending ? "Archiving..." : confirming ? "Archive for good?" : "Archive item"}
        </button>
        {error && (
          <p className={`${form.message} ${form.error}`} role="status">
            {error}
          </p>
        )}
      </div>
    </div>
  );
}
