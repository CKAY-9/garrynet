import { Suspense } from "react";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import styles from "../admin.module.scss";

export default function CommunityPage() {
  return (
    <Suspense>
      <Community />
    </Suspense>
  );
}

async function Community() {
  await requirePermission(Permission.AdminAccess | Permission.CommunityManagement);

  return (
    <section className={styles.section} aria-labelledby="community-heading">
      <h2 id="community-heading" className={styles.sectionTitle}>
        Community
      </h2>
      {/* TODO: edit community details. */}
      <p className={styles.placeholder}>Coming soon.</p>
    </section>
  );
}
