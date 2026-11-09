import type { ServerStatus } from "@/lib/servers";
import styles from "./servers.module.scss";

const LABELS: Record<ServerStatus, string> = {
  unlinked: "Not linked",
  waiting: "Waiting for heartbeat",
  online: "Online",
  offline: "Offline",
};

export function StatusBadge({ status }: { status: ServerStatus }) {
  return (
    <span className={`${styles.status} ${styles[status]}`}>
      <span className={styles.dot} aria-hidden />
      {LABELS[status]}
    </span>
  );
}
