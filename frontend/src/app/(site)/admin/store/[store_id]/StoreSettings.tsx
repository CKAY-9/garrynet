"use client";

import { useState, useTransition, type FormEvent } from "react";
import { CURRENCIES, MAX_MESSAGE, MAX_NAME, currencyLabel, isCurrency } from "@/lib/storeEnums";
import type { ItemStore, StoreDetails } from "@/lib/types";
import form from "../../community/editor.module.scss";
import { Field } from "../CreateStoreForm";
import { updateStore } from "../actions";
import styles from "../store.module.scss";

type Draft = { enabled: boolean; currency: string; override_name: string; store_message: string };

function toDraft(s: StoreDetails): Draft {
  return {
    enabled: s.enabled,
    currency: s.currency,
    override_name: s.override_name ?? "",
    store_message: s.store_message ?? "",
  };
}

function toDetails(d: Draft): StoreDetails {
  return {
    enabled: d.enabled,
    currency: d.currency.trim().toLowerCase(),
    override_name: d.override_name.trim() || null,
    store_message: d.store_message.trim() || null,
  };
}

type Status = { result: "idle" } | { result: "saved" } | { result: "error"; message: string };

/** A store's settings (not its grantable usergroups). Changes apply when saved. */
export function StoreSettings({ store, hasItems }: { store: ItemStore; hasItems: boolean }) {
  const [saved, setSaved] = useState(() => toDetails(toDraft(store)));
  const [draft, setDraft] = useState(() => toDraft(store));
  const [status, setStatus] = useState<Status>({ result: "idle" });
  const [saving, startSaving] = useTransition();
  const details = toDetails(draft);
  const changed = JSON.stringify(details) !== JSON.stringify(saved);
  const patch = (p: Partial<Draft>) => setDraft((d) => ({ ...d, ...p }));

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startSaving(async () => {
      const res = await updateStore(store.id, details);
      if (res.ok) {
        const next = toDraft(res.data);
        setSaved(toDetails(next));
        setDraft(next);
        setStatus({ result: "saved" });
      } else {
        setStatus({ result: "error", message: res.message });
      }
    });
  }

  return (
    <form className={form.group} onSubmit={onSubmit}>
      <div className={styles.grid}>
        <Field label="Store name" hint="Optional. Defaults to the server's name.">
          <input className={form.input} value={draft.override_name} onChange={(e) => patch({ override_name: e.target.value })} maxLength={MAX_NAME} />
        </Field>
        <Field
          label="Currency"
          hint={hasItems ? "Fixed while the store has items: their prices are in this currency. Archive them to change it." : "Every item in the store uses it."}
        >
          <select className={form.input} value={draft.currency} onChange={(e) => patch({ currency: e.target.value })} disabled={hasItems} required>
            {/* A store set up before the list keeps its currency as an option. */}
            {[...(isCurrency(store.currency) ? [] : [store.currency]), ...CURRENCIES].map((c) => (
              <option key={c} value={c}>
                {currencyLabel(c)}
              </option>
            ))}
          </select>
        </Field>
      </div>
      <Field label="Message" hint="Optional. Shown at the top of the store page.">
        <textarea className={`${form.input} ${form.textarea}`} value={draft.store_message} onChange={(e) => patch({ store_message: e.target.value })} maxLength={MAX_MESSAGE} rows={3} />
      </Field>
      <label className={styles.checkbox}>
        <input type="checkbox" checked={draft.enabled} onChange={(e) => patch({ enabled: e.target.checked })} />
        <span>
          <span className={form.label}>Enabled</span>
          <span className={form.hint}>When off, the store is hidden and nothing can be bought.</span>
        </span>
      </label>
      <div className={styles.inlineActions}>
        <button type="submit" className={form.save} disabled={!changed || saving}>
          {saving ? "Saving..." : "Save"}
        </button>
        <button
          type="button"
          className={form.discard}
          onClick={() => {
            setDraft(toDraft(saved));
            setStatus({ result: "idle" });
          }}
          disabled={!changed || saving}
        >
          Discard changes
        </button>
        <p className={form.message} role="status">
          {status.result === "error" && <span className={form.error}>{status.message}</span>}
          {status.result === "saved" && !changed && "Saved."}
          {status.result !== "error" && changed && !saving && "Unsaved changes."}
        </p>
      </div>
    </form>
  );
}
