import Link from "next/link";
import { dateFormat, fullFormat, timeAgo } from "@/lib/time";
import type { Punishment, UserProfile } from "@/lib/types";
import styles from "./Punishments.module.scss";

function Time({ value }: { value: string }) {
  const date = new Date(value);
  return (
    <time dateTime={value} title={fullFormat.format(date)}>
      {timeAgo(date)}
    </time>
  );
}

function Until({ punishment }: { punishment: Pick<Punishment, "ends"> }) {
  if (!punishment.ends) return <>permanently</>;
  const ends = new Date(punishment.ends);
  return (
    <>
      until{" "}
      <time dateTime={punishment.ends} title={fullFormat.format(ends)}>
        {fullFormat.format(ends)}
      </time>
    </>
  );
}

/** A staff member's name, or `fallback` when the backend doesn't say who it was. */
function StaffName({ user, fallback }: { user: Punishment["issuer"]; fallback: string }) {
  return user ? <Link href={`/users/${user.id}`}>{user.username}</Link> : <>{fallback}</>;
}

/** Only the website issues punishments, so a missing issuer is a deleted account. */
const ISSUER_GONE = "a deleted account";
/** The backend shell can lift them too. */
const LIFTER_GONE = "the server console or a deleted account";

/**
 * A ban in effect, shown to everyone on the profile: when it was issued, until when, and why.
 * Never who issued it.
 */
export function BanNotice({ ban }: { ban: NonNullable<UserProfile["ban"]> }) {
  const issued = new Date(ban.created_at);
  return (
    <div className={styles.banNotice} role="status">
      <strong>Banned</strong> <Until punishment={ban} />
      <p className={styles.reason}>{ban.reason}</p>
      <p className={styles.meta}>
        Banned on{" "}
        <time dateTime={ban.created_at} title={fullFormat.format(issued)}>
          {dateFormat.format(issued)}
        </time>{" "}
        ({timeAgo(issued)})
      </p>
    </div>
  );
}

/**
 * The user's suspension in effect and, for staff, their full punishment history. A ban in
 * effect is shown to everyone by [`BanNotice`] instead. `staff` is false when users view their
 * own profile; the backend then sends only what's in effect, without staff names.
 */
export function Punishments({ punishments, staff }: { punishments: Punishment[]; staff: boolean }) {
  const current = punishments.filter((p) => p.in_effect && p.kind === "SUSPEND");
  if (current.length === 0 && !(staff && punishments.length > 0)) return null;

  return (
    <section className={styles.punishments} aria-label="Punishments">
      {current.map((p) => (
        <div key={p.id} className={styles.suspendNotice} role="status">
          <strong>Suspended</strong> <Until punishment={p} />
          <p className={styles.reason}>{p.reason}</p>
          {staff && (
            <p className={styles.meta}>
              Issued by <StaffName user={p.issuer} fallback={ISSUER_GONE} /> <Time value={p.created_at} />
            </p>
          )}
        </div>
      ))}

      {staff && (
        <details className={styles.history}>
          <summary>Punishment history ({punishments.length})</summary>
          <ol className={styles.historyList}>
            {punishments.map((p) => (
              <li key={p.id}>
                <span className={styles.historyKind}>{p.kind === "BAN" ? "Ban" : "Suspension"}</span>{" "}
                <Until punishment={p} />, by <StaffName user={p.issuer} fallback={ISSUER_GONE} /> <Time value={p.created_at} />
                <span className={styles.historyState}>
                  {p.in_effect ? (
                    " · In effect"
                  ) : p.lifted_at ? (
                    <>
                      {" "}
                      · Lifted by <StaffName user={p.lifted_by} fallback={LIFTER_GONE} /> <Time value={p.lifted_at} />
                      {p.lift_reason && `: ${p.lift_reason}`}
                    </>
                  ) : (
                    " · Ended"
                  )}
                </span>
                <p className={styles.reason}>{p.reason}</p>
              </li>
            ))}
          </ol>
        </details>
      )}
    </section>
  );
}
