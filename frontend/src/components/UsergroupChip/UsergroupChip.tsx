import type { CSSProperties } from "react";
import type { UsergroupBadge } from "@/lib/types";
import styles from "./UsergroupChip.module.scss";

/** A small pill showing a usergroup's name in its color, animated by its effect. */
export function UsergroupChip({ group }: { group: UsergroupBadge }) {
  return (
    <span
      className={`${styles.chip} ${styles[group.effect] ?? ""}`}
      style={{ "--chip-color": group.color } as CSSProperties}
    >
      {group.icon_url ? (
        // Plain <img>: group icons can be hosted anywhere, so next/image can't allowlist them.
        // eslint-disable-next-line @next/next/no-img-element
        <img className={styles.icon} src={group.icon_url} alt="" width={14} height={14} />
      ) : (
        <span className={styles.dot} aria-hidden />
      )}
      {group.name}
    </span>
  );
}
