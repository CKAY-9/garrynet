"use client";

import { useEffect, useState } from "react";
import { GMOD_CALLBACKS, GMOD_QUEUE, type GmodCallback } from "./gmod";
import styles from "./LoadingScreen.module.scss";

type Progress = { status: string; value: number };

/**
 * The progress bar and its status text.
 *
 * - `live`: driven by the JavaScript functions Garry's Mod calls while the player
 *   connects (`SetStatusChanged`, `SetFilesTotal`, ...). Calls made before this component
 *   mounted are queued by `<GmodBridge>` and replayed.
 * - `demo`: loops through a fake connection, for previews.
 */
export function LoadingProgress({ mode }: { mode: "live" | "demo" }) {
  const [progress, setProgress] = useState<Progress>({ status: "Connecting...", value: 0 });
  // Bumped each time the demo restarts, so the bar starts over instead of shrinking back.
  const [cycle, setCycle] = useState(0);

  useEffect(() => (mode === "live" ? listenToGmod(setProgress) : runDemo(setProgress, setCycle)), [mode]);

  const percent = Math.round(progress.value * 100);
  return (
    <div className={styles.progress}>
      <div className={styles.progressText}>
        <span className={styles.status}>{progress.status}</span>
        <span className={styles.percent}>{percent}%</span>
      </div>
      <div
        className={styles.track}
        role="progressbar"
        aria-label="Loading"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={percent}
      >
        <div key={cycle} className={styles.fill} style={{ width: `${progress.value * 100}%` }} />
      </div>
    </div>
  );
}

/**
 * How far along each connection stage is. Garry's Mod doesn't report overall progress, so
 * the bar moves by stage, and by file count while downloading (between `DOWNLOAD_START`
 * and `DOWNLOAD_END`).
 */
const STAGES: [match: string, value: number][] = [
  ["retrieving server info", 0.04],
  ["mounting", 0.08],
  ["workshop", 0.1],
  ["workshop complete", 0.75],
  ["no files need", 0.75],
  ["sending client info", 0.82],
  ["client info sent", 0.86],
  ["received all lua files", 0.94],
  ["starting lua", 0.97],
];
const DOWNLOAD_START = 0.1;
const DOWNLOAD_END = 0.75;

function stageValue(status: string): number {
  const lower = status.toLowerCase();
  let value = 0;
  for (const [match, stage] of STAGES) if (lower.includes(match)) value = Math.max(value, stage);
  return value;
}

function listenToGmod(set: (p: Progress) => void): () => void {
  let status = "Connecting...";
  let value = 0;
  let filesTotal = 0;
  let filesNeeded = 0;

  // Only ever moves forward: stages and file counts can arrive out of order.
  const advance = (to: number) => {
    value = Math.min(1, Math.max(value, to));
    set({ status, value });
  };
  const files = () => {
    if (filesTotal <= 0) return 0;
    const done = (filesTotal - filesNeeded) / filesTotal;
    return DOWNLOAD_START + (DOWNLOAD_END - DOWNLOAD_START) * done;
  };

  const handlers: Record<GmodCallback, (...args: unknown[]) => void> = {
    GameDetails: () => advance(0.02),
    SetFilesTotal: (total) => {
      filesTotal = Number(total) || 0;
      filesNeeded = filesTotal;
      advance(files());
    },
    SetFilesNeeded: (needed) => {
      filesNeeded = Number(needed) || 0;
      advance(files());
    },
    DownloadingFile: (file) => {
      status = `Downloading ${String(file)}`;
      advance(files());
    },
    SetStatusChanged: (next) => {
      status = String(next);
      advance(stageValue(status));
    },
  };

  const queued = window[GMOD_QUEUE] ?? [];
  for (const name of GMOD_CALLBACKS) window[name] = handlers[name];
  for (const [name, args] of queued) handlers[name]?.(...args);
  window[GMOD_QUEUE] = [];

  return () => {
    for (const name of GMOD_CALLBACKS) delete window[name];
  };
}

/** A fake connection, played on a loop. */
const DEMO: [status: string, value: number, ms: number][] = [
  ["Retrieving server info...", 0.04, 900],
  ["Mounting addons", 0.08, 900],
  ["Downloading materials/garrynet/hud/icons.png", 0.2, 700],
  ["Downloading models/props_garrynet/crate01.mdl", 0.34, 700],
  ["Downloading sound/garrynet/ambience/city.ogg", 0.5, 700],
  ["Downloading maps/graphs/rp_downtown.ain", 0.62, 700],
  ["Workshop Complete", 0.75, 900],
  ["Sending client info...", 0.82, 800],
  ["Client info sent!", 0.86, 700],
  ["Received all Lua files we needed!", 0.94, 900],
  ["Starting Lua...", 1, 1800],
];

function runDemo(set: (p: Progress) => void, setCycle: (f: (c: number) => number) => void): () => void {
  let step = 0;
  let timer: ReturnType<typeof setTimeout>;
  const next = () => {
    const [status, value, ms] = DEMO[step];
    set({ status, value });
    step = (step + 1) % DEMO.length;
    timer = setTimeout(() => {
      if (step === 0) setCycle((c) => c + 1);
      next();
    }, ms);
  };
  next();
  return () => clearTimeout(timer);
}
