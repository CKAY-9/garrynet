"use client";

import { useActionState, useState } from "react";
import { LoadingScreen, LoadingScreenFrame, previewServer } from "@/components/LoadingScreen";
import type { LoadingScreenPlayer, LoadingScreenStyle } from "@/lib/types";
import { saveLoadingScreenStyle } from "./actions";
import styles from "./editor.module.scss";

const STYLES: { value: LoadingScreenStyle; label: string; description: string }[] = [
  { value: "aero", label: "Aero", description: "Glossy Frutiger Aero, like a classic Garry's Mod loading screen." },
  { value: "sleek", label: "Sleek", description: "Minimal, clean and dim." },
];

/** Style picker with a live preview. The preview changes on selection; Save applies it. */
export function LoadingScreenEditor({
  saved,
  player,
}: {
  saved: LoadingScreenStyle;
  /** Fake player for the preview, built on the server so both render the same dates. */
  player: LoadingScreenPlayer;
}) {
  const [state, save, saving] = useActionState(saveLoadingScreenStyle, { style: saved, result: "idle" });
  const [selected, setSelected] = useState(saved);
  const changed = selected !== state.style;

  return (
    <form action={save} className={styles.editor}>
      <fieldset className={styles.styles}>
        <legend className={styles.legend}>Style</legend>
        {STYLES.map(({ value, label, description }) => (
          <label key={value} className={styles.option}>
            <input
              type="radio"
              name="style"
              value={value}
              checked={selected === value}
              onChange={() => setSelected(value)}
            />
            <span className={styles.optionText}>
              <span className={styles.optionLabel}>
                {label}
                {state.style === value && <span className={styles.current}>Current</span>}
              </span>
              <span className={styles.optionDescription}>{description}</span>
            </span>
          </label>
        ))}
      </fieldset>

      <div className={styles.actions}>
        <button type="submit" className={styles.save} disabled={!changed || saving}>
          {saving ? "Saving..." : "Save"}
        </button>
        <p className={styles.message} role="status">
          {state.result === "error" && <span className={styles.error}>{state.message}</span>}
          {state.result === "saved" && !changed && "Saved."}
          {changed && !saving && "Unsaved changes."}
        </p>
      </div>

      <LoadingScreenFrame label="Loading screen preview">
        <LoadingScreen style={selected} server={previewServer} player={player} mode="demo" />
      </LoadingScreenFrame>
    </form>
  );
}
