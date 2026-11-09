import { Suspense } from "react";
import { SupportNav, TicketFilterForm, TicketRows } from "@/components/Support";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import { getAllTickets, readTicketFilters, type SearchParams } from "@/lib/support";
import styles from "@/components/Support/Support.module.scss";

export default function SupportAdminPage({ searchParams }: PageProps<"/support/admin">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <AllTickets searchParams={searchParams} />
    </Suspense>
  );
}

/** /support/admin: every ticket, for the Support Team. */
async function AllTickets({ searchParams }: { searchParams: SearchParams }) {
  await requirePermission(Permission.SupportTeam);
  const filters = await readTicketFilters(searchParams);
  const tickets = await getAllTickets(filters);
  const filtered = Object.values(filters).some((v) => v);

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1>Support</h1>
        <p>Every ticket in the community, newest activity first.</p>
      </header>
      <SupportNav support active="admin" />
      <TicketFilterForm filters={filters} action="/support/admin" />
      <section className={styles.section} aria-labelledby="tickets-heading">
        <div className={styles.sectionHead}>
          <h2 id="tickets-heading">All tickets</h2>
          <span>
            {tickets.length} {tickets.length === 1 ? "ticket" : "tickets"}
          </span>
        </div>
        <TicketRows tickets={tickets} showAuthor empty={filtered ? "No tickets match these filters." : "No tickets yet."} />
      </section>
    </div>
  );
}
