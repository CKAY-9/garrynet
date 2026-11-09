"use client";

import { useState, useTransition, type FormEvent, type ReactNode } from "react";
import type { ForumSection } from "@/lib/types";
// The same form styles as the other admin editors.
import form from "../community/editor.module.scss";
import { createCategory, createSection, deleteCategory, deleteSection } from "./actions";
import styles from "./forum.module.scss";

/** Limits enforced by the backend (src/routes/forum.rs). */
const MAX_NAME = 64;
const MAX_DESCRIPTION = 256;
const DEFAULT_ACCENT = "#3b82f6";

function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className={form.field}>
      <span className={form.label}>{label}</span>
      {children}
      {hint && <span className={form.hint}>{hint}</span>}
    </label>
  );
}

const POSITION_HINT = "Listed highest first; -1 is always last.";

/** Submit state shared by the create forms; `onDone` resets the fields after a success. */
function useSubmit(onDone: () => void) {
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  function submit(request: () => Promise<{ ok: true; data: { name: string } } | { ok: false; message: string }>) {
    startTransition(async () => {
      const res = await request();
      if (res.ok) {
        setError(null);
        setSaved(`Created ${res.data.name}.`);
        onDone();
      } else {
        setSaved(null);
        setError(res.message);
      }
    });
  }
  return { error, saved, pending, submit };
}

function Status({ error, saved, idle }: { error: string | null; saved: string | null; idle?: string }) {
  return (
    <p className={form.message} role="status">
      {error ? <span className={form.error}>{error}</span> : (saved ?? idle)}
    </p>
  );
}

export function CreateSectionForm() {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [accent, setAccent] = useState(DEFAULT_ACCENT);
  const [iconUrl, setIconUrl] = useState("");
  const [position, setPosition] = useState("0");
  const { error, saved, pending, submit } = useSubmit(() => {
    setName("");
    setDescription("");
    setIconUrl("");
  });

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    submit(() =>
      createSection({
        name: name.trim(),
        description: description.trim(),
        accent: accent.trim().toLowerCase(),
        icon_url: iconUrl.trim() || null,
        position: Number(position) || 0,
      }),
    );
  }

  const pickerValue = /^#[0-9a-f]{6}$/i.test(accent) ? accent : DEFAULT_ACCENT;

  return (
    <form className={form.group} onSubmit={onSubmit}>
      <div className={styles.grid}>
        <Field label="Name">
          <input className={form.input} value={name} onChange={(e) => setName(e.target.value)} maxLength={MAX_NAME} placeholder="Community" required />
        </Field>
        <Field label="Accent color">
          <span className={styles.colorRow}>
            <input
              className={form.input}
              value={accent}
              onChange={(e) => setAccent(e.target.value)}
              pattern="#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})"
              required
            />
            <input className={styles.colorPicker} type="color" value={pickerValue} onChange={(e) => setAccent(e.target.value)} aria-label="Pick accent color" />
          </span>
        </Field>
      </div>
      <Field label="Description">
        <input className={form.input} value={description} onChange={(e) => setDescription(e.target.value)} maxLength={MAX_DESCRIPTION} />
      </Field>
      <div className={styles.grid}>
        <Field label="Icon URL" hint="Optional.">
          <input className={form.input} type="url" value={iconUrl} onChange={(e) => setIconUrl(e.target.value)} placeholder="https://example.com/icon.png" />
        </Field>
        <Field label="Position" hint={POSITION_HINT}>
          <input className={form.input} type="number" min={-1} step={1} value={position} onChange={(e) => setPosition(e.target.value)} required />
        </Field>
      </div>
      <div className={styles.actions}>
        <button type="submit" className={form.save} disabled={pending || !name.trim()}>
          {pending ? "Creating..." : "Create section"}
        </button>
        <Status error={error} saved={saved} />
      </div>
    </form>
  );
}

export function CreateCategoryForm({ sections }: { sections: Pick<ForumSection, "id" | "name">[] }) {
  const [sectionId, setSectionId] = useState(String(sections[0]?.id ?? ""));
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [locked, setLocked] = useState(false);
  const [position, setPosition] = useState("0");
  const { error, saved, pending, submit } = useSubmit(() => {
    setName("");
    setDescription("");
    setLocked(false);
  });

  if (sections.length === 0) {
    return <p className={styles.empty}>Create a section first; categories belong to one.</p>;
  }

  // Keep the selection valid if its section was deleted.
  const selected = sections.some((s) => String(s.id) === sectionId) ? sectionId : String(sections[0].id);

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    submit(() =>
      createCategory({
        section_id: Number(selected),
        name: name.trim(),
        description: description.trim(),
        locked,
        position: Number(position) || 0,
      }),
    );
  }

  return (
    <form className={form.group} onSubmit={onSubmit}>
      <div className={styles.grid}>
        <Field label="Section">
          <select className={form.input} value={selected} onChange={(e) => setSectionId(e.target.value)}>
            {sections.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
        </Field>
        <Field label="Name">
          <input className={form.input} value={name} onChange={(e) => setName(e.target.value)} maxLength={MAX_NAME} placeholder="General Discussion" required />
        </Field>
      </div>
      <Field label="Description">
        <input className={form.input} value={description} onChange={(e) => setDescription(e.target.value)} maxLength={MAX_DESCRIPTION} />
      </Field>
      <div className={styles.grid}>
        <Field label="Position" hint={POSITION_HINT}>
          <input className={form.input} type="number" min={-1} step={1} value={position} onChange={(e) => setPosition(e.target.value)} required />
        </Field>
        <label className={styles.checkbox}>
          <input type="checkbox" checked={locked} onChange={(e) => setLocked(e.target.checked)} />
          <span>
            <span className={form.label}>Locked</span>
            <span className={form.hint}>No new threads or replies.</span>
          </span>
        </label>
      </div>
      <div className={styles.actions}>
        <button type="submit" className={form.save} disabled={pending || !name.trim()}>
          {pending ? "Creating..." : "Create category"}
        </button>
        <Status error={error} saved={saved} />
      </div>
    </form>
  );
}

/** Delete button that asks for a second click, since deleting takes every thread with it. */
export function DeleteButton({ kind, id, name }: { kind: "section" | "category"; id: number; name: string }) {
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();

  function onClick() {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    startTransition(async () => {
      const res = kind === "section" ? await deleteSection(id) : await deleteCategory(id);
      if (!res.ok) {
        setError(res.message);
        setConfirming(false);
      }
    });
  }

  return (
    <span className={styles.deleteWrap}>
      {error && <span className={form.error}>{error}</span>}
      <button
        type="button"
        className={confirming ? styles.dangerConfirm : styles.danger}
        onClick={onClick}
        onBlur={() => setConfirming(false)}
        disabled={pending}
        aria-label={confirming ? `Confirm deleting ${name}` : `Delete ${name}`}
      >
        {pending ? "Deleting..." : confirming ? "Delete everything in it?" : "Delete"}
      </button>
    </span>
  );
}
