"use client";

import { useState, useTransition, type FormEvent } from "react";
import Link from "next/link";
import { Markdown } from "@/components/Markdown";
import { legalHref } from "@/lib/legalPages";
import { dateFormat } from "@/lib/time";
import type { LegalDocument, LegalSlug } from "@/lib/types";
import { loadLegalTemplate, saveLegalDocument } from "./actions";
// The form controls look like the community editor's.
import styles from "../community/editor.module.scss";
import legal from "./legal.module.scss";

/** Limits enforced by the backend (src/routes/legal.rs). */
const MAX_TITLE = 100;
const MAX_BODY = 100_000;

type Draft = { title: string; body: string };
type Status = { result: "idle" } | { result: "saved" } | { result: "error"; message: string };

const toDraft = (d: Draft): Draft => ({ title: d.title.trim(), body: d.body.trim() });
const same = (a: Draft, b: Draft) => a.title === b.title && a.body === b.body;
const bySlug = <T,>(documents: LegalDocument[], f: (d: LegalDocument) => T) =>
  Object.fromEntries(documents.map((d) => [d.slug, f(d)])) as Record<LegalSlug, T>;

/**
 * A tab per legal document, each with its title and Markdown body. Unsaved edits are kept
 * while switching tabs; each document saves on its own.
 */
export function LegalEditor({ documents }: { documents: LegalDocument[] }) {
  const [saved, setSaved] = useState(() => bySlug(documents, (d) => d));
  const [drafts, setDrafts] = useState(() => bySlug(documents, toDraft));
  const [active, setActive] = useState<LegalSlug>(documents[0].slug);
  const [preview, setPreview] = useState(false);
  const [status, setStatus] = useState<Status>({ result: "idle" });
  const [pending, startTransition] = useTransition();

  const current = saved[active];
  const draft = drafts[active];
  const isChanged = (slug: LegalSlug) => !same(toDraft(drafts[slug]), toDraft(saved[slug]));
  const changed = isChanged(active);

  const update = (patch: Partial<Draft>) => setDrafts((d) => ({ ...d, [active]: { ...d[active], ...patch } }));

  function switchTo(slug: LegalSlug) {
    setActive(slug);
    setStatus({ result: "idle" });
  }

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const slug = active;
    startTransition(async () => {
      const res = await saveLegalDocument(slug, toDraft(drafts[slug]));
      if (res.ok) {
        // Show the document as the backend stored it (trimmed, any HTML escaped).
        setSaved((s) => ({ ...s, [slug]: res.data }));
        setDrafts((d) => ({ ...d, [slug]: toDraft(res.data) }));
        setStatus({ result: "saved" });
      } else {
        setStatus({ result: "error", message: res.message });
      }
    });
  }

  function loadTemplate() {
    const slug = active;
    startTransition(async () => {
      const res = await loadLegalTemplate(slug);
      if (res.ok) {
        setDrafts((d) => ({ ...d, [slug]: res.data }));
        setStatus({ result: "idle" });
      } else {
        setStatus({ result: "error", message: res.message });
      }
    });
  }

  function discard() {
    setDrafts((d) => ({ ...d, [active]: toDraft(saved[active]) }));
    setStatus({ result: "idle" });
  }

  return (
    <div className={styles.editor}>
      <div className={legal.tabs} role="tablist" aria-label="Documents">
        {documents.map(({ slug }) => (
          <button
            key={slug}
            type="button"
            role="tab"
            id={`legal-tab-${slug}`}
            aria-selected={slug === active}
            aria-controls="legal-panel"
            className={`${legal.tab} ${slug === active ? legal.active : ""}`}
            onClick={() => switchTo(slug)}
          >
            {saved[slug].title}
            {isChanged(slug) && <span className={legal.dot} aria-label="(unsaved changes)" />}
          </button>
        ))}
      </div>

      <form id="legal-panel" role="tabpanel" aria-labelledby={`legal-tab-${active}`} className={styles.editor} onSubmit={onSubmit}>
        {current.last_updated === null ? (
          <p className={legal.notice}>
            This is still GarryNet&apos;s default template. Replace the text in [square brackets], remove anything that
            doesn&apos;t apply to your community, and check it against the law where you and your players live.
          </p>
        ) : (
          <p className={styles.hint}>Last updated {dateFormat.format(new Date(current.last_updated))}.</p>
        )}

        <label className={styles.field}>
          <span className={styles.label}>Title</span>
          <input
            className={styles.input}
            value={draft.title}
            onChange={(e) => update({ title: e.target.value })}
            maxLength={MAX_TITLE}
            required
          />
        </label>

        <div className={styles.field}>
          <div className={legal.bodyHeader}>
            <label className={styles.label} htmlFor="legal-body">
              Text (Markdown)
            </label>
            <div className={legal.modes} role="group" aria-label="View">
              <button type="button" className={`${legal.mode} ${!preview ? legal.modeActive : ""}`} onClick={() => setPreview(false)} aria-pressed={!preview}>
                Write
              </button>
              <button type="button" className={`${legal.mode} ${preview ? legal.modeActive : ""}`} onClick={() => setPreview(true)} aria-pressed={preview}>
                Preview
              </button>
            </div>
          </div>
          {preview ? (
            <div className={legal.preview}>
              {draft.body.trim() ? <Markdown>{draft.body}</Markdown> : <p className={styles.hint}>Nothing to preview.</p>}
            </div>
          ) : (
            <textarea
              id="legal-body"
              className={`${styles.input} ${styles.textarea} ${legal.body}`}
              value={draft.body}
              onChange={(e) => update({ body: e.target.value })}
              maxLength={MAX_BODY}
              rows={24}
              required
            />
          )}
          <span className={styles.hint}>
            Headings, lists, links and tables work. HTML is shown as plain text. The date on the page changes each time you save.
          </span>
        </div>

        <div className={styles.actions}>
          <button type="submit" className={styles.save} disabled={!changed || pending}>
            {pending ? "Saving..." : "Save"}
          </button>
          <button type="button" className={styles.discard} onClick={discard} disabled={!changed || pending}>
            Discard changes
          </button>
          <button type="button" className={styles.discard} onClick={loadTemplate} disabled={pending}>
            Load default template
          </button>
          <p className={styles.message} role="status">
            {status.result === "error" && <span className={styles.error}>{status.message}</span>}
            {status.result === "saved" && !changed && "Saved."}
            {status.result !== "error" && changed && !pending && "Unsaved changes."}
          </p>
          <Link href={legalHref(active)} className={styles.view}>
            View page
          </Link>
        </div>
      </form>
    </div>
  );
}
