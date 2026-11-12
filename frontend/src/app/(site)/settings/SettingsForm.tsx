"use client";

import { useId, useState, useTransition, type FormEvent } from "react";
import type { ProfileSettings } from "@/lib/types";
import { saveProfileSettings } from "./actions";
import styles from "./settings.module.scss";

/** Must match `MAX_BIO` in the backend (src/routes/users.rs). */
const MAX_BIO = 1000;

type Status = { result: "idle" } | { result: "saved" } | { result: "error"; message: string };

/** The user's bio and profile privacy. Changes apply when saved. */
export function SettingsForm({ settings }: { settings: ProfileSettings }) {
  const [saved, setSaved] = useState(settings);
  const [bio, setBio] = useState(settings.bio);
  const [isPrivate, setIsPrivate] = useState(settings.private);
  const [status, setStatus] = useState<Status>({ result: "idle" });
  const [saving, startTransition] = useTransition();
  const ids = useId();

  const draft: ProfileSettings = { bio: bio.trim(), private: isPrivate };
  const changed = draft.bio !== saved.bio || draft.private !== saved.private;

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startTransition(async () => {
      const res = await saveProfileSettings(draft);
      if (res.ok) {
        const next = { bio: res.data.bio, private: res.data.private };
        setSaved(next);
        setBio(next.bio);
        setIsPrivate(next.private);
        setStatus({ result: "saved" });
      } else {
        setStatus({ result: "error", message: res.message });
      }
    });
  }

  function discard() {
    setBio(saved.bio);
    setIsPrivate(saved.private);
    setStatus({ result: "idle" });
  }

  return (
    <form className={styles.form} onSubmit={onSubmit}>
      <label className={styles.field} htmlFor={`${ids}-bio`}>
        <span className={styles.label}>Biography</span>
        <textarea
          id={`${ids}-bio`}
          className={styles.textarea}
          value={bio}
          onChange={(e) => setBio(e.target.value)}
          maxLength={MAX_BIO}
          rows={6}
          placeholder="Tell the community a bit about yourself."
          aria-describedby={`${ids}-bio-hint`}
        />
        <span id={`${ids}-bio-hint`} className={styles.hint}>
          Shown on your profile as plain text. {bio.length.toLocaleString("en")}/{MAX_BIO.toLocaleString("en")}
        </span>
      </label>

      <label className={styles.switchRow} htmlFor={`${ids}-private`}>
        <span className={styles.switchText}>
          <span className={styles.label}>Private profile</span>
          <span id={`${ids}-private-hint`} className={styles.hint}>
            Hides your bio, reputation, threads and posts from other users. Staff can still see them, and your
            posts still show in their threads.
          </span>
        </span>
        <input
          id={`${ids}-private`}
          className={styles.switch}
          type="checkbox"
          role="switch"
          checked={isPrivate}
          onChange={(e) => setIsPrivate(e.target.checked)}
          aria-describedby={`${ids}-private-hint`}
        />
      </label>

      <div className={styles.actions}>
        <button type="submit" className={styles.save} disabled={!changed || saving}>
          {saving ? "Saving..." : "Save"}
        </button>
        <button type="button" className={styles.discard} onClick={discard} disabled={!changed || saving}>
          Discard
        </button>
        <p className={styles.message} role="status">
          {status.result === "error" && <span className={styles.error}>{status.message}</span>}
          {status.result === "saved" && !changed && "Saved."}
          {status.result !== "error" && changed && !saving && "Unsaved changes."}
        </p>
      </div>
    </form>
  );
}
