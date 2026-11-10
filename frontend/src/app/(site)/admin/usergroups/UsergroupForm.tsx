"use client";

import { ImageUrlInput } from "@/components/Upload";
import { UsergroupChip } from "@/components/UsergroupChip";
import { PERMISSION_LABELS, hasPermission } from "@/lib/permissions";
import type { UsergroupDetails, UsergroupEffect } from "@/lib/types";
// The same form styles as the community and server editors.
import form from "../community/editor.module.scss";
import { Field } from "../servers/ServerForm";
import servers from "../servers/servers.module.scss";
import styles from "./usergroups.module.scss";

/** Limit enforced by the backend (src/routes/usergroups.rs). */
const MAX_NAME = 32;

const DEFAULT_COLOR = "#9ba1a6";

const EFFECTS: { value: UsergroupEffect; label: string }[] = [
  { value: "none", label: "None" },
  { value: "halo", label: "Halo" },
  { value: "rainbow", label: "Rainbow" },
  { value: "flash", label: "Flash" },
];

/** Form state: every text field is a string while editing. */
export type UsergroupDraft = Omit<UsergroupDetails, "icon_url"> & { icon_url: string };

export const EMPTY_DRAFT: UsergroupDraft = { name: "", effect: "none", color: DEFAULT_COLOR, permissions: 0, icon_url: "" };

export function toDraft(d: UsergroupDetails): UsergroupDraft {
  return { name: d.name, effect: d.effect, color: d.color, permissions: d.permissions, icon_url: d.icon_url ?? "" };
}

/** What gets sent to the backend: trimmed, a blank icon URL `null`. */
export function toDetails(d: UsergroupDraft): UsergroupDetails {
  return {
    name: d.name.trim(),
    effect: d.effect,
    color: d.color.trim().toLowerCase(),
    permissions: d.permissions,
    icon_url: d.icon_url.trim() || null,
  };
}

/**
 * Every editable usergroup field. Permissions the editor doesn't hold can't be toggled
 * (the backend refuses those changes too).
 */
export function UsergroupFields({
  draft,
  onChange,
  held,
  lockName = false,
}: {
  draft: UsergroupDraft;
  onChange: (patch: Partial<UsergroupDraft>) => void;
  /** The editor's own permissions. */
  held: number;
  /** For the default groups, whose name is fixed. */
  lockName?: boolean;
}) {
  // `<input type="color">` only takes #rrggbb.
  const pickerValue = /^#[0-9a-f]{6}$/i.test(draft.color) ? draft.color : DEFAULT_COLOR;
  const preview = toDetails(draft);

  return (
    <>
      <div className={styles.preview}>
        <span className={form.label}>Preview</span>
        <UsergroupChip group={{ id: 0, ...preview, name: preview.name || "Usergroup" }} />
      </div>

      <div className={servers.fieldGrid}>
        <Field label="Name" hint={lockName ? "Default groups can't be renamed." : undefined}>
          <input
            className={form.input}
            value={draft.name}
            onChange={(e) => onChange({ name: e.target.value })}
            maxLength={MAX_NAME}
            pattern=".*\D.*"
            title="Must contain something other than digits"
            readOnly={lockName}
            required
          />
        </Field>
        <Field label="Color">
          <span className={servers.colorRow}>
            <input
              className={form.input}
              value={draft.color}
              onChange={(e) => onChange({ color: e.target.value })}
              pattern="#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})"
              placeholder={DEFAULT_COLOR}
              required
            />
            <input
              className={servers.colorPicker}
              type="color"
              value={pickerValue}
              onChange={(e) => onChange({ color: e.target.value })}
              aria-label="Pick color"
            />
          </span>
        </Field>
        <Field label="Effect">
          <select
            className={form.input}
            value={draft.effect}
            onChange={(e) => onChange({ effect: e.target.value as UsergroupEffect })}
          >
            {EFFECTS.map(({ value, label }) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </Field>
      </div>

      <Field label="Icon URL" hint="Optional. Shown small beside the name; square images work best.">
        <ImageUrlInput
          className={form.input}
          value={draft.icon_url}
          onChange={(url) => onChange({ icon_url: url })}
          placeholder="https://example.com/icon.png"
        />
      </Field>

      <fieldset className={styles.permissions}>
        <legend className={form.label}>Permissions</legend>
        <p className={form.hint}>You can only grant or revoke permissions you hold yourself.</p>
        <div className={styles.permGrid}>
          {PERMISSION_LABELS.map(({ flag, label }) => {
            const canChange = hasPermission(held, flag);
            return (
              <label key={flag} className={`${styles.permOption} ${canChange ? "" : styles.permLocked}`}>
                <input
                  type="checkbox"
                  checked={hasPermission(draft.permissions, flag)}
                  disabled={!canChange}
                  onChange={(e) => onChange({ permissions: e.target.checked ? draft.permissions | flag : draft.permissions & ~flag })}
                />
                <span>{label}</span>
                <code className={styles.bit}>0x{flag.toString(16)}</code>
              </label>
            );
          })}
        </div>
      </fieldset>
    </>
  );
}
