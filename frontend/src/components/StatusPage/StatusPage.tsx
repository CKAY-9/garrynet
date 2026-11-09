import type { ReactNode } from "react";
import styles from "./StatusPage.module.scss";

/** A centered message for error pages such as 401 and 403. */
export function StatusPage({ code, title, children }: { code: number; title: string; children: ReactNode }) {
  return (
    <div className={styles.page}>
      <p className={styles.code}>{code}</p>
      <h1 className={styles.title}>{title}</h1>
      {children}
    </div>
  );
}
