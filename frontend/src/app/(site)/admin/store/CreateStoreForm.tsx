"use client";

import { useState, useTransition, type FormEvent, type ReactNode } from "react";
import { CURRENCIES, MAX_MESSAGE, MAX_NAME, currencyLabel } from "@/lib/storeEnums";
import type { Server } from "@/lib/types";
// The same form styles as the other admin editors.
import form from "../community/editor.module.scss";
import { createStore } from "./actions";
import styles from "./store.module.scss";

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className={form.field}>
      <span className={form.label}>{label}</span>
      {children}
      {hint && <span className={form.hint}>{hint}</span>}
    </label>
  );
}

/** Creates a store for a server that doesn't have one yet, then opens it. */
export function CreateStoreForm({ servers }: { servers: Pick<Server, "id" | "name">[] }) {
  const [serverId, setServerId] = useState(String(servers[0]?.id ?? ""));
  const [currency, setCurrency] = useState("usd");
  const [name, setName] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  if (servers.length === 0) {
    return <p className={styles.empty}>Every registered server already has a store. Register a server first to add another.</p>;
  }
  const selected = servers.some((s) => String(s.id) === serverId) ? serverId : String(servers[0].id);

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startTransition(async () => {
      // Redirects to the new store on success.
      const res = await createStore(Number(selected), {
        enabled: true,
        currency: currency.trim().toLowerCase(),
        override_name: name.trim() || null,
        store_message: message.trim() || null,
      });
      if (!res.ok) setError(res.message);
    });
  }

  return (
    <form className={form.group} onSubmit={onSubmit}>
      <div className={styles.grid}>
        <Field label="Server">
          <select className={form.input} value={selected} onChange={(e) => setServerId(e.target.value)}>
            {servers.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Currency" hint="Every item in the store uses it. It can't change once the store has items.">
          <select className={form.input} value={currency} onChange={(e) => setCurrency(e.target.value)} required>
            {CURRENCIES.map((c) => (
              <option key={c} value={c}>
                {currencyLabel(c)}
              </option>
            ))}
          </select>
        </Field>
      </div>
      <Field label="Store name" hint="Optional. Defaults to the server's name.">
        <input className={form.input} value={name} onChange={(e) => setName(e.target.value)} maxLength={MAX_NAME} />
      </Field>
      <Field label="Message" hint="Optional. Shown at the top of the store page.">
        <textarea className={`${form.input} ${form.textarea}`} value={message} onChange={(e) => setMessage(e.target.value)} maxLength={MAX_MESSAGE} rows={2} />
      </Field>
      <div className={styles.inlineActions}>
        <button type="submit" className={form.save} disabled={pending}>
          {pending ? "Creating..." : "Create store"}
        </button>
        <p className={form.message} role="status">
          {error ? <span className={form.error}>{error}</span> : "The store starts enabled with no items."}
        </p>
      </div>
    </form>
  );
}
