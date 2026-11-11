"use client";

import { useId, useRef, useState, useTransition, type FormEvent } from "react";
import { fullFormat } from "@/lib/time";
import type { Punishment, PunishmentKind } from "@/lib/types";
import { liftPunishment, punishUser } from "./actions";
import styles from "./Punishments.module.scss";

/** Must match `MAX_REASON` in the backend (src/routes/punishments.rs). */
const MAX_REASON = 500;

const HOUR = 60 * 60 * 1000;
const DAY = 24 * HOUR;

/** Preset lengths; `null` is permanent (bans only), `"custom"` asks for a date. */
type Duration = { label: string; ms: number | null } | { label: string; ms: "custom" };
const DURATIONS: Record<PunishmentKind, Duration[]> = {
  BAN: [
    { label: "Permanent", ms: null },
    { label: "1 day", ms: DAY },
    { label: "3 days", ms: 3 * DAY },
    { label: "1 week", ms: 7 * DAY },
    { label: "2 weeks", ms: 14 * DAY },
    { label: "1 month", ms: 30 * DAY },
    { label: "Until a date...", ms: "custom" },
  ],
  SUSPEND: [
    { label: "1 hour", ms: HOUR },
    { label: "12 hours", ms: 12 * HOUR },
    { label: "1 day", ms: DAY },
    { label: "3 days", ms: 3 * DAY },
    { label: "1 week", ms: 7 * DAY },
    { label: "2 weeks", ms: 14 * DAY },
    { label: "1 month", ms: 30 * DAY },
    { label: "Until a date...", ms: "custom" },
  ],
};

const COPY: Record<PunishmentKind, { verb: string; undo: string; effect: string; defaultDuration: number }> = {
  BAN: {
    verb: "Ban",
    undo: "Unban",
    effect:
      "They lose every permission and can only use support, where they can appeal. If the community keeps banned users off its game servers, they can't join those either.",
    defaultDuration: 0,
  },
  SUSPEND: {
    verb: "Suspend",
    undo: "Unsuspend",
    effect: "They can still browse and buy, but can't post on the forums or send messages until it ends.",
    defaultDuration: 2,
  },
};

type Props = {
  userId: number;
  username: string;
  /** The ban and suspension in effect, if any. */
  ban: Punishment | null;
  suspension: Punishment | null;
};

/**
 * Ban/unban and suspend/unsuspend buttons for staff on a user's profile. Each opens a dialog
 * asking for what's needed: a reason and a length to punish, an optional note to lift.
 */
export function PunishmentControls({ userId, username, ban, suspension }: Props) {
  return (
    <>
      <PunishmentButton userId={userId} username={username} kind="BAN" active={ban} />
      <PunishmentButton userId={userId} username={username} kind="SUSPEND" active={suspension} />
    </>
  );
}

function PunishmentButton({
  userId,
  username,
  kind,
  active,
}: {
  userId: number;
  username: string;
  kind: PunishmentKind;
  active: Punishment | null;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const copy = COPY[kind];
  const label = active ? copy.undo : copy.verb;
  // Remounted on every open so the form starts empty.
  const [opened, setOpened] = useState(0);

  function open() {
    setOpened((n) => n + 1);
    dialog.current?.showModal();
  }

  return (
    <>
      <button type="button" className={active ? styles.undoButton : styles.dangerButton} onClick={open}>
        {label}
      </button>
      <dialog ref={dialog} className={styles.dialog} aria-label={`${label} ${username}`}>
        {opened > 0 &&
          (active ? (
            <LiftForm key={opened} userId={userId} username={username} punishment={active} close={() => dialog.current?.close()} />
          ) : (
            <IssueForm key={opened} userId={userId} username={username} kind={kind} close={() => dialog.current?.close()} />
          ))}
      </dialog>
    </>
  );
}

/** `datetime-local` value for a date, in the browser's time zone. */
function localInputValue(date: Date): string {
  const offset = date.getTimezoneOffset() * 60 * 1000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 16);
}

function IssueForm({
  userId,
  username,
  kind,
  close,
}: {
  userId: number;
  username: string;
  kind: PunishmentKind;
  close: () => void;
}) {
  const copy = COPY[kind];
  const durations = DURATIONS[kind];
  const [reason, setReason] = useState("");
  const [duration, setDuration] = useState(copy.defaultDuration);
  const [until, setUntil] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const ids = useId();
  const chosen = durations[duration];

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    let ends: string | null;
    if (chosen.ms === "custom") {
      const date = new Date(until);
      if (Number.isNaN(date.getTime()) || date.getTime() <= Date.now()) {
        setError("Pick a date and time in the future.");
        return;
      }
      ends = date.toISOString();
    } else {
      ends = chosen.ms === null ? null : new Date(Date.now() + chosen.ms).toISOString();
    }
    setError(null);
    startTransition(async () => {
      const res = await punishUser(userId, kind, reason.trim(), ends);
      if (res.ok) close();
      else setError(res.message);
    });
  }

  return (
    <form className={styles.form} onSubmit={onSubmit}>
      <h2 className={styles.dialogTitle}>
        {copy.verb} {username}
      </h2>
      <p className={styles.dialogText}>{copy.effect}</p>

      <label className={styles.field} htmlFor={`${ids}-reason`}>
        <span className={styles.label}>Reason</span>
        <textarea
          id={`${ids}-reason`}
          className={styles.input}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={MAX_REASON}
          rows={3}
          required
          autoFocus
        />
        <span className={styles.hint}>They&apos;re told this in a notification.</span>
      </label>

      <label className={styles.field} htmlFor={`${ids}-duration`}>
        <span className={styles.label}>Length</span>
        <select
          id={`${ids}-duration`}
          className={styles.input}
          value={duration}
          onChange={(e) => setDuration(Number(e.target.value))}
        >
          {durations.map((d, i) => (
            <option key={d.label} value={i}>
              {d.label}
            </option>
          ))}
        </select>
      </label>

      {chosen.ms === "custom" && (
        <label className={styles.field} htmlFor={`${ids}-until`}>
          <span className={styles.label}>Ends</span>
          <input
            id={`${ids}-until`}
            className={styles.input}
            type="datetime-local"
            value={until}
            min={localInputValue(new Date())}
            onChange={(e) => setUntil(e.target.value)}
            required
          />
          <span className={styles.hint}>In your time zone.</span>
        </label>
      )}

      {error && (
        <p className={styles.error} role="alert">
          {error}
        </p>
      )}

      <div className={styles.dialogActions}>
        <button type="button" className={styles.cancel} onClick={close} disabled={pending}>
          Cancel
        </button>
        <button type="submit" className={styles.confirm} disabled={pending || !reason.trim()}>
          {pending ? "Saving..." : `${copy.verb} ${username}`}
        </button>
      </div>
    </form>
  );
}

function LiftForm({
  userId,
  username,
  punishment,
  close,
}: {
  userId: number;
  username: string;
  punishment: Punishment;
  close: () => void;
}) {
  const copy = COPY[punishment.kind];
  const [reason, setReason] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [pending, startTransition] = useTransition();
  const id = useId();

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    startTransition(async () => {
      const res = await liftPunishment(userId, punishment.kind, reason.trim());
      if (res.ok) close();
      else setError(res.message);
    });
  }

  return (
    <form className={styles.form} onSubmit={onSubmit}>
      <h2 className={styles.dialogTitle}>
        {copy.undo} {username}
      </h2>
      <p className={styles.dialogText}>
        {punishment.kind === "BAN" ? "Banned" : "Suspended"}{" "}
        {punishment.ends ? `until ${fullFormat.format(new Date(punishment.ends))}` : "permanently"}. Reason:{" "}
        {punishment.reason}
      </p>

      <label className={styles.field} htmlFor={id}>
        <span className={styles.label}>Note (optional)</span>
        <textarea
          id={id}
          className={styles.input}
          value={reason}
          onChange={(e) => setReason(e.target.value)}
          maxLength={MAX_REASON}
          rows={2}
          placeholder="e.g. Appeal accepted"
          autoFocus
        />
        <span className={styles.hint}>Kept in their punishment history; they aren&apos;t shown it.</span>
      </label>

      {error && (
        <p className={styles.error} role="alert">
          {error}
        </p>
      )}

      <div className={styles.dialogActions}>
        <button type="button" className={styles.cancel} onClick={close} disabled={pending}>
          Cancel
        </button>
        <button type="submit" className={styles.confirmNeutral} disabled={pending}>
          {pending ? "Saving..." : `${copy.undo} ${username}`}
        </button>
      </div>
    </form>
  );
}
