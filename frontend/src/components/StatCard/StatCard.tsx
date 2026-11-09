import styles from "./StatCard.module.scss";

type Props = {
  label: string;
  /** Omit for a placeholder card (e.g. a stat that isn't implemented yet). */
  value?: number | string;
  /** Small text under the value. */
  detail?: string;
  /** Marks the card as not implemented yet. */
  todo?: boolean;
};

/** A single dashboard statistic. */
export function StatCard({ label, value, detail, todo }: Props) {
  return (
    <div className={`${styles.card} ${todo ? styles.todo : ""}`}>
      <dt className={styles.label}>
        {label}
        {todo && <span className={styles.badge}>TODO</span>}
      </dt>
      <dd className={styles.value}>
        {value === undefined ? "—" : typeof value === "number" ? value.toLocaleString("en") : value}
      </dd>
      {detail && <dd className={styles.detail}>{detail}</dd>}
    </div>
  );
}

export function StatCardSkeleton({ label }: { label: string }) {
  return (
    <div className={styles.card} aria-busy>
      <dt className={styles.label}>{label}</dt>
      <dd className={`${styles.value} ${styles.skeleton}`} />
    </div>
  );
}
