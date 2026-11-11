import { Suspense } from "react";
import { Breadcrumbs, FaqForm } from "@/components/Support";
import { requirePermission } from "@/lib/auth";
import { SUPPORT_TEAM } from "@/lib/support";
import styles from "@/components/Support/Support.module.scss";

export default function NewFaqPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <NewFaq />
    </Suspense>
  );
}

/** /support/faq/new: writes an FAQ. Support Team only. */
async function NewFaq() {
  await requirePermission(SUPPORT_TEAM);
  return (
    <div className={styles.page}>
      <Breadcrumbs items={[{ href: "/support", label: "Support" }, { href: "/support/faq", label: "FAQs" }, { label: "New FAQ" }]} />
      <header className={styles.header}>
        <h1>New FAQ</h1>
        <p>Answer a question the community keeps asking.</p>
      </header>
      <FaqForm />
    </div>
  );
}
