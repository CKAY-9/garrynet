import { Suspense } from "react";
import { requirePermission } from "@/lib/auth";
import { getCommunity } from "@/lib/community";
import { Permission } from "@/lib/permissions";
import styles from "../admin.module.scss";
import { CommunityEditor } from "./CommunityEditor";

export default function CommunityPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <CommunitySettings />
    </Suspense>
  );
}

async function CommunitySettings() {
  await requirePermission(Permission.AdminAccess | Permission.CommunityManagement);
  const community = await getCommunity();

  return (
    <section className={styles.section} aria-labelledby="community-heading">
      <h2 id="community-heading" className={styles.sectionTitle}>
        Community
      </h2>
      {community ? (
        <CommunityEditor community={community} />
      ) : (
        <p className={styles.placeholder}>The community couldn&apos;t be loaded. Try again shortly.</p>
      )}
    </section>
  );
}
