"use client";

import { useState, useTransition, type FormEvent } from "react";
import { MAX_MESSAGE } from "@/lib/storeEnums";
import { fullFormat } from "@/lib/time";
import type { DiscountDetails, StoreDiscount } from "@/lib/types";
import form from "../../community/editor.module.scss";
import { Field } from "../CreateStoreForm";
import { createDiscount, deleteDiscount, updateDiscount } from "../actions";
import styles from "../store.module.scss";

/** An ISO timestamp as a `datetime-local` value (local time, minutes). */
function toLocalInput(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** A `datetime-local` value as an ISO timestamp, or `null` when blank. */
function fromLocalInput(value: string): string | null {
  if (!value) return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/** Creates a store-wide discount, or edits `discount` when given. */
export function DiscountForm({ storeId, discount, onDone }: { storeId: number; discount?: StoreDiscount; onDone?: () => void }) {
  const [percent, setPercent] = useState(String(discount?.discount ?? 10));
  const [message, setMessage] = useState(discount?.message ?? "");
  const [startsAt, setStartsAt] = useState(toLocalInput(discount?.starts_at ?? null));
  const [endsAt, setEndsAt] = useState(toLocalInput(discount?.ends_at ?? null));
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [pending, startTransition] = useTransition();

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const details: DiscountDetails = {
      discount: Number(percent) || 0,
      message: message.trim(),
      starts_at: fromLocalInput(startsAt),
      ends_at: fromLocalInput(endsAt),
    };
    startTransition(async () => {
      const res = discount ? await updateDiscount(storeId, discount.id, details) : await createDiscount(storeId, details);
      if (res.ok) {
        setError(null);
        if (discount) {
          onDone?.();
        } else {
          setSaved(true);
          setMessage("");
          setStartsAt("");
          setEndsAt("");
        }
      } else {
        setSaved(false);
        setError(res.message);
      }
    });
  }

  return (
    <form className={form.group} onSubmit={onSubmit} aria-label={discount ? "Edit discount" : "New discount"}>
      <div className={styles.grid}>
        <Field label="Percent off" hint="Added to each item's own discount; the total is capped at 100%.">
          <input className={form.input} type="number" min={0} max={100} step={1} value={percent} onChange={(e) => setPercent(e.target.value)} required />
        </Field>
        <Field label="Message" hint="Shown on the store page, e.g. “Summer sale”.">
          <input className={form.input} value={message} onChange={(e) => setMessage(e.target.value)} maxLength={MAX_MESSAGE} />
        </Field>
      </div>
      <div className={styles.grid}>
        <Field label="Starts" hint="Optional. Active immediately when blank.">
          <input className={form.input} type="datetime-local" value={startsAt} onChange={(e) => setStartsAt(e.target.value)} />
        </Field>
        <Field label="Ends" hint="Optional. Indefinite when blank.">
          <input className={form.input} type="datetime-local" value={endsAt} onChange={(e) => setEndsAt(e.target.value)} />
        </Field>
      </div>
      <div className={styles.inlineActions}>
        <button type="submit" className={form.save} disabled={pending}>
          {pending ? "Saving..." : discount ? "Save changes" : "Create discount"}
        </button>
        {onDone && (
          <button type="button" className={form.discard} onClick={onDone} disabled={pending}>
            Cancel
          </button>
        )}
        <p className={form.message} role="status">
          {error ? <span className={form.error}>{error}</span> : saved ? "Created." : ""}
        </p>
      </div>
    </form>
  );
}

/** One discount with its window, and Edit / Delete. */
export function DiscountRow({ storeId, discount }: { storeId: number; discount: StoreDiscount }) {
  const [editing, setEditing] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  if (editing) {
    return <DiscountForm key={discount.last_updated ?? discount.created_at} storeId={storeId} discount={discount} onDone={() => setEditing(false)} />;
  }

  function onDelete() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    startTransition(async () => {
      const res = await deleteDiscount(storeId, discount.id);
      if (!res.ok) {
        setError(res.message);
        setConfirming(false);
      }
    });
  }

  const window =
    discount.starts_at || discount.ends_at
      ? [
          discount.starts_at ? `from ${fullFormat.format(new Date(discount.starts_at))}` : "",
          discount.ends_at ? `until ${fullFormat.format(new Date(discount.ends_at))}` : "indefinitely",
        ]
          .filter(Boolean)
          .join(" ")
      : "always";

  return (
    <li className={styles.card} style={{ borderLeftColor: discount.active ? "#22c55e" : "rgb(127 127 127 / 0.4)" }}>
      <span className={styles.cardBody}>
        <span className={styles.cardTitle}>
          {discount.discount}% off
          <span className={`${styles.badge} ${discount.active ? styles.on : ""}`}>{discount.active ? "Active" : "Inactive"}</span>
          {discount.message && <span className={styles.cardMeta}>{discount.message}</span>}
        </span>
        <span className={styles.cardMeta}>Applies {window}</span>
      </span>
      <span className={styles.tools}>
        {error && <span className={form.error}>{error}</span>}
        <button type="button" className={styles.tool} onClick={() => setEditing(true)} disabled={pending}>
          Edit
        </button>
        <button
          type="button"
          className={confirming ? styles.dangerConfirm : styles.danger}
          onClick={onDelete}
          onBlur={() => setConfirming(false)}
          disabled={pending}
        >
          {pending ? "Deleting..." : confirming ? "Really delete?" : "Delete"}
        </button>
      </span>
    </li>
  );
}
