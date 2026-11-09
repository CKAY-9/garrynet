import { Suspense } from "react";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import styles from "../admin.module.scss";

export default function LoadingScreenPage() {
  return (
    <Suspense>
      <LoadingScreen />
    </Suspense>
  );
}

async function LoadingScreen() {
  await requirePermission(Permission.AdminAccess | Permission.ServerManagement);

  return (
    <section className={styles.section} aria-labelledby="loading-screen-heading">
      <h2 id="loading-screen-heading" className={styles.sectionTitle}>
        Loading Screen
      </h2>
      {/* TODO: change the style of the loading screen. */}
      <p className={styles.placeholder}>Coming soon.</p>
    </section>
  );
}
