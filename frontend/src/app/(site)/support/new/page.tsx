import { Suspense } from "react";
import { unauthorized } from "next/navigation";
import { Breadcrumbs, NewTicketForm } from "@/components/Support";
import { getCurrentUser } from "@/lib/auth";
import { MAX_ACTIVE_TICKETS } from "@/lib/supportEnums";
import styles from "@/components/Support/Support.module.scss";

export default function NewTicketPage() {
  return (
    // Reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <NewTicket />
    </Suspense>
  );
}

/** /support/new: opens a ticket. Login required. */
async function NewTicket() {
  const user = await getCurrentUser();
  if (!user) unauthorized();
  return (
    <div className={styles.page}>
      <Breadcrumbs items={[{ href: "/support", label: "Support" }, { label: "New ticket" }]} />
      <header className={styles.header}>
        <h1>New ticket</h1>
        <p>
          Tell the support team what&apos;s going on. You can have up to {MAX_ACTIVE_TICKETS} tickets in progress at once.
        </p>
      </header>
      <NewTicketForm />
    </div>
  );
}
