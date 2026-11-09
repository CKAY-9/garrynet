"use client";

import type { ReactNode } from "react";
import { Avatar } from "@/components/Avatar";
import type { ServerDetails } from "@/lib/types";
// The same form styles as the community editor.
import form from "../community/editor.module.scss";
import styles from "./servers.module.scss";

/** Limits enforced by the backend (src/routes/servers.rs). */
export const MAX_NAME = 64;
export const MAX_DESCRIPTION = 2000;
const MAX_ADDRESS = 255;

export const DEFAULT_COLOR = "#3b82f6";

/** Form state: every field is a string while editing. */
export type ServerDraft = {
  name: string;
  description: string;
  splash_url: string;
  icon_url: string;
  color: string;
  connection_address: string;
};

export function toDraft(d: ServerDetails): ServerDraft {
  return {
    name: d.name,
    description: d.description,
    splash_url: d.splash_url ?? "",
    icon_url: d.icon_url ?? "",
    color: d.color,
    connection_address: d.connection_address,
  };
}

/** What gets sent to the backend: trimmed, blank URLs `null`. */
export function toDetails(d: ServerDraft): ServerDetails {
  return {
    name: d.name.trim(),
    description: d.description.trim(),
    splash_url: d.splash_url.trim() || null,
    icon_url: d.icon_url.trim() || null,
    color: d.color.trim().toLowerCase(),
    connection_address: d.connection_address.trim(),
  };
}

/** Every editable server field. */
export function ServerFields({
  draft,
  onChange,
}: {
  draft: ServerDraft;
  onChange: (patch: Partial<ServerDraft>) => void;
}) {
  // `<input type="color">` only takes #rrggbb.
  const pickerValue = /^#[0-9a-f]{6}$/i.test(draft.color) ? draft.color : DEFAULT_COLOR;

  return (
    <>
      <div className={form.identity}>
        <span className={styles.iconPreview} style={{ borderColor: draft.color }}>
          <Avatar name={draft.name} url={draft.icon_url.trim() || null} size={64} />
        </span>
        <div className={form.fields}>
          <Field label="Name">
            <input className={form.input} value={draft.name} onChange={(e) => onChange({ name: e.target.value })} maxLength={MAX_NAME} required />
          </Field>
          <Field label="Icon URL" hint="Square images work best. Shown on the loading screen.">
            <input className={form.input} type="url" value={draft.icon_url} onChange={(e) => onChange({ icon_url: e.target.value })} placeholder="https://example.com/icon.png" />
          </Field>
        </div>
      </div>

      <Field label="Description">
        <textarea
          className={`${form.input} ${form.textarea}`}
          value={draft.description}
          onChange={(e) => onChange({ description: e.target.value })}
          maxLength={MAX_DESCRIPTION}
          rows={3}
        />
      </Field>

      <div className={styles.fieldGrid}>
        <Field label="Accent color" hint="Used on the loading screen.">
          <span className={styles.colorRow}>
            <input
              className={form.input}
              value={draft.color}
              onChange={(e) => onChange({ color: e.target.value })}
              pattern="#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})"
              placeholder={DEFAULT_COLOR}
              required
            />
            <input
              className={styles.colorPicker}
              type="color"
              value={pickerValue}
              onChange={(e) => onChange({ color: e.target.value })}
              aria-label="Pick accent color"
            />
          </span>
        </Field>
        <Field label="Connection address" hint="What players connect to, e.g. 203.0.113.5:27015.">
          <input
            className={form.input}
            value={draft.connection_address}
            onChange={(e) => onChange({ connection_address: e.target.value })}
            maxLength={MAX_ADDRESS}
            pattern="\S*"
            placeholder="203.0.113.5:27015"
          />
        </Field>
      </div>

      <Field label="Splash image URL" hint="A wide image shown on the website (not on the loading screen).">
        <input className={form.input} type="url" value={draft.splash_url} onChange={(e) => onChange({ splash_url: e.target.value })} placeholder="https://example.com/splash.jpg" />
      </Field>
      {draft.splash_url.trim() && (
        <div
          className={form.splashPreview}
          style={{ backgroundImage: `url(${JSON.stringify(draft.splash_url.trim())})` }}
          role="img"
          aria-label="Splash image preview"
        />
      )}
    </>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className={form.field}>
      <span className={form.label}>{label}</span>
      {children}
      {hint && <span className={form.hint}>{hint}</span>}
    </label>
  );
}
