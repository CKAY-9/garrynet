import { Suspense } from "react";
import { requirePermission } from "@/lib/auth";
import { getLegalDocument } from "@/lib/legal";
import { legalPages } from "@/lib/legalPages";
import { Permission } from "@/lib/permissions";
import type { LegalDocument } from "@/lib/types";
import styles from "../admin.module.scss";
import { LegalEditor } from "./LegalEditor";

export default function LegalPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <LegalSettings />
    </Suspense>
  );
}

async function LegalSettings() {
  await requirePermission(Permission.AdminAccess | Permission.CommunityManagement);
  const documents = (await Promise.all(legalPages.map((p) => getLegalDocument(p.slug)))).filter(
    (d): d is LegalDocument => d !== null,
  );

  return (
    <section className={styles.section} aria-labelledby="legal-heading">
      <h2 id="legal-heading" className={styles.sectionTitle}>
        Legal
      </h2>
      <p className={styles.note}>
        Your terms, privacy policy and refund policy, linked from every page, the login button and the store. GarryNet
        fills them with templates to start from; they aren&apos;t legal advice.
      </p>
      {documents.length > 0 ? (
        <LegalEditor documents={documents} />
      ) : (
        <p className={styles.placeholder}>The documents couldn&apos;t be loaded. Try again shortly.</p>
      )}
    </section>
  );
}
