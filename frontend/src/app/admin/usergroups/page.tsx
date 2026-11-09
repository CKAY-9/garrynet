import { Suspense } from "react";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import styles from "../admin.module.scss";

export default function UsergroupsPage() {
  return (
    <Suspense>
      <Usergroups />
    </Suspense>
  );
}

async function Usergroups() {
  await requirePermission(Permission.AdminAccess | Permission.UsergroupManagement);

  return (
    <section className={styles.section} aria-labelledby="usergroups-heading">
      <h2 id="usergroups-heading" className={styles.sectionTitle}>
        Usergroups
      </h2>
      {/* TODO: create, edit and remove usergroups. */}
      <p className={styles.placeholder}>Coming soon.</p>
    </section>
  );
}
