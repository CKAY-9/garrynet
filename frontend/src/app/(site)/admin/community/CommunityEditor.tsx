"use client";

import { useState, useTransition, type FormEvent, type ReactNode } from "react";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { ImageUrlInput } from "@/components/Upload";
import { defaultCommunityLogo } from "@/lib/config";
import type { Community, CommunityDetails, KeyFigure } from "@/lib/types";
import { saveCommunity } from "./actions";
import styles from "./editor.module.scss";

/** Limits enforced by the backend (src/routes/community.rs). */
const MAX_NAME = 64;
const MAX_DESCRIPTION = 4000;
const MAX_LINKS = 16;
const MAX_KEY_FIGURES = 32;
const MAX_ROLE = 64;
const MAX_FIGURE_DESCRIPTION = 500;

/** List items carry a local key so React keeps inputs in place when rows move or are removed. */
type Keyed<T> = T & { key: number };
type FigureDraft = Keyed<Omit<KeyFigure, "user_id"> & { user_id: string }>;
type Draft = {
  name: string;
  description: string;
  logo_url: string;
  splash_background: string;
  external_links: Keyed<{ url: string }>[];
  key_figures: FigureDraft[];
};

let nextKey = 0;
const key = () => nextKey++;

function toDraft(c: CommunityDetails): Draft {
  return {
    name: c.name,
    description: c.description,
    logo_url: c.logo_url ?? "",
    splash_background: c.splash_background ?? "",
    external_links: c.external_links.map((url) => ({ key: key(), url })),
    key_figures: c.key_figures.map((f) => ({ ...f, user_id: f.user_id?.toString() ?? "", key: key() })),
  };
}

const emptyFigure = (): FigureDraft => ({
  key: key(),
  user_id: "",
  name: "",
  profile_picture: "",
  role: "",
  description: "",
});

/** What gets sent to the backend: blank links dropped, blank URLs `null`. */
function toDetails(d: Draft): CommunityDetails {
  return {
    name: d.name.trim(),
    description: d.description.trim(),
    logo_url: d.logo_url.trim() || null,
    splash_background: d.splash_background.trim() || null,
    external_links: d.external_links.map((l) => l.url.trim()).filter(Boolean),
    key_figures: d.key_figures.map(({ user_id, name, profile_picture, role, description }) => ({
      ...(user_id.trim() ? { user_id: Number(user_id) } : {}),
      name: name.trim(),
      profile_picture: profile_picture.trim(),
      role: role.trim(),
      description: description.trim(),
    })),
  };
}

type Status = { result: "idle" } | { result: "saved" } | { result: "error"; message: string };

/** Form for every editable community detail. Changes apply when saved. */
export function CommunityEditor({ community }: { community: Community }) {
  const [saved, setSaved] = useState(() => toDetails(toDraft(community)));
  const [draft, setDraft] = useState(() => toDraft(saved));
  const [status, setStatus] = useState<Status>({ result: "idle" });
  const [saving, startSaving] = useTransition();
  const details = toDetails(draft);
  const changed = JSON.stringify(details) !== JSON.stringify(saved);

  const update = (patch: Partial<Draft>) => setDraft((d) => ({ ...d, ...patch }));

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startSaving(async () => {
      const res = await saveCommunity(details);
      if (res.ok) {
        // Show the details as the backend normalized them.
        const next = toDetails(toDraft(res.community));
        setSaved(next);
        setDraft(toDraft(next));
        setStatus({ result: "saved" });
      } else {
        setStatus({ result: "error", message: res.message });
      }
    });
  }

  function reset() {
    setDraft(toDraft(saved));
    setStatus({ result: "idle" });
  }

  // Links
  const setLink = (k: number, url: string) =>
    update({ external_links: draft.external_links.map((l) => (l.key === k ? { ...l, url } : l)) });
  const removeLink = (k: number) => update({ external_links: draft.external_links.filter((l) => l.key !== k) });
  const addLink = () => update({ external_links: [...draft.external_links, { key: key(), url: "" }] });

  // Key figures
  const setFigure = (k: number, patch: Partial<FigureDraft>) =>
    update({ key_figures: draft.key_figures.map((f) => (f.key === k ? { ...f, ...patch } : f)) });
  const removeFigure = (k: number) => update({ key_figures: draft.key_figures.filter((f) => f.key !== k) });
  const addFigure = () => update({ key_figures: [...draft.key_figures, emptyFigure()] });
  const moveFigure = (index: number, by: -1 | 1) => {
    const figures = [...draft.key_figures];
    [figures[index], figures[index + by]] = [figures[index + by], figures[index]];
    update({ key_figures: figures });
  };

  return (
    <form className={styles.editor} onSubmit={onSubmit}>
      <fieldset className={styles.group}>
        <legend className={styles.legend}>Details</legend>

        <div className={styles.identity}>
          <Avatar name={draft.name} url={draft.logo_url.trim() || defaultCommunityLogo} size={72} />
          <div className={styles.fields}>
            <Field label="Name">
              <input
                className={styles.input}
                value={draft.name}
                onChange={(e) => update({ name: e.target.value })}
                maxLength={MAX_NAME}
                required
              />
            </Field>
            <Field label="Logo URL" hint="Paste a link or upload an image. Square images work best. Leave empty to use the default GarryNet logo.">
              <ImageUrlInput
                className={styles.input}
                value={draft.logo_url}
                onChange={(url) => update({ logo_url: url })}
                placeholder="https://example.com/logo.png"
              />
            </Field>
          </div>
        </div>

        <Field label="Description" hint="Shown at the top of the homepage.">
          <textarea
            className={`${styles.input} ${styles.textarea}`}
            value={draft.description}
            onChange={(e) => update({ description: e.target.value })}
            maxLength={MAX_DESCRIPTION}
            rows={4}
          />
        </Field>

        <Field label="Splash background URL" hint="A wide image shown behind the homepage header. Paste a link or upload one.">
          <ImageUrlInput
            className={styles.input}
            value={draft.splash_background}
            onChange={(url) => update({ splash_background: url })}
            placeholder="https://example.com/banner.jpg"
          />
        </Field>
        {draft.splash_background.trim() && (
          <div
            className={styles.splashPreview}
            style={{ backgroundImage: `url(${JSON.stringify(draft.splash_background.trim())})` }}
            role="img"
            aria-label="Splash background preview"
          />
        )}
      </fieldset>

      <fieldset className={styles.group}>
        <legend className={styles.legend}>External links</legend>
        <p className={styles.hint}>Discord, Steam group, socials and so on. Shown on the homepage.</p>
        {draft.external_links.map((link, i) => (
          <div key={link.key} className={styles.row}>
            <input
              className={styles.input}
              type="url"
              value={link.url}
              onChange={(e) => setLink(link.key, e.target.value)}
              placeholder="https://discord.gg/..."
              aria-label={`Link ${i + 1}`}
            />
            <button type="button" className={styles.iconButton} onClick={() => removeLink(link.key)} aria-label={`Remove link ${i + 1}`}>
              <CloseIcon />
            </button>
          </div>
        ))}
        <button type="button" className={styles.add} onClick={addLink} disabled={draft.external_links.length >= MAX_LINKS}>
          + Add link
        </button>
      </fieldset>

      <fieldset className={styles.group}>
        <legend className={styles.legend}>Key figures</legend>
        <p className={styles.hint}>Owners, staff and other notable members, in the order shown on the homepage.</p>
        {draft.key_figures.map((figure, i) => (
          <div key={figure.key} className={styles.figure}>
            <div className={styles.figureHeader}>
              <Avatar name={figure.name} url={figure.profile_picture.trim() || null} size={40} shape="circle" />
              <span className={styles.figureTitle}>{figure.name.trim() || "New key figure"}</span>
              <div className={styles.figureActions}>
                <button type="button" className={styles.iconButton} onClick={() => moveFigure(i, -1)} disabled={i === 0} aria-label={`Move ${figure.name || "key figure"} up`}>
                  <ArrowIcon up />
                </button>
                <button type="button" className={styles.iconButton} onClick={() => moveFigure(i, 1)} disabled={i === draft.key_figures.length - 1} aria-label={`Move ${figure.name || "key figure"} down`}>
                  <ArrowIcon />
                </button>
                <button type="button" className={styles.iconButton} onClick={() => removeFigure(figure.key)} aria-label={`Remove ${figure.name || "key figure"}`}>
                  <CloseIcon />
                </button>
              </div>
            </div>
            <div className={styles.figureFields}>
              <Field label="Name">
                <input className={styles.input} value={figure.name} onChange={(e) => setFigure(figure.key, { name: e.target.value })} maxLength={MAX_NAME} required />
              </Field>
              <Field label="Role">
                <input className={styles.input} value={figure.role} onChange={(e) => setFigure(figure.key, { role: e.target.value })} maxLength={MAX_ROLE} placeholder="Owner" />
              </Field>
              <Field label="Picture" hint="Paste a link or upload an image.">
                <ImageUrlInput className={styles.input} value={figure.profile_picture} onChange={(url) => setFigure(figure.key, { profile_picture: url })} placeholder="https://..." />
              </Field>
              <Field label="GarryNet user ID" hint="Optional. Links to their profile.">
                <input className={styles.input} type="number" min={1} step={1} value={figure.user_id} onChange={(e) => setFigure(figure.key, { user_id: e.target.value })} />
              </Field>
            </div>
            <Field label="Description">
              <textarea
                className={`${styles.input} ${styles.textarea}`}
                value={figure.description}
                onChange={(e) => setFigure(figure.key, { description: e.target.value })}
                maxLength={MAX_FIGURE_DESCRIPTION}
                rows={2}
              />
            </Field>
          </div>
        ))}
        <button type="button" className={styles.add} onClick={addFigure} disabled={draft.key_figures.length >= MAX_KEY_FIGURES}>
          + Add key figure
        </button>
      </fieldset>

      <div className={styles.actions}>
        <button type="submit" className={styles.save} disabled={!changed || saving}>
          {saving ? "Saving..." : "Save"}
        </button>
        <button type="button" className={styles.discard} onClick={reset} disabled={!changed || saving}>
          Discard changes
        </button>
        <p className={styles.message} role="status">
          {status.result === "error" && <span className={styles.error}>{status.message}</span>}
          {status.result === "saved" && !changed && "Saved."}
          {status.result !== "error" && changed && !saving && "Unsaved changes."}
        </p>
        <Link href="/" className={styles.view}>
          View homepage
        </Link>
      </div>
    </form>
  );
}

function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className={styles.field}>
      <span className={styles.label}>{label}</span>
      {children}
      {hint && <span className={styles.hint}>{hint}</span>}
    </label>
  );
}

const CloseIcon = () => (
  <svg viewBox="0 0 24 24" width={16} height={16} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" aria-hidden>
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
);

const ArrowIcon = ({ up = false }: { up?: boolean }) => (
  <svg viewBox="0 0 24 24" width={16} height={16} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden style={up ? { transform: "rotate(180deg)" } : undefined}>
    <path d="M12 5v14M6 13l6 6 6-6" />
  </svg>
);
