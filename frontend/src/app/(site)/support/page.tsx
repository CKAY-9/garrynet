import { Suspense } from "react";
import Link from "next/link";
import { FaqGrid, SupportNav, TicketFilterForm, TicketRows } from "@/components/Support";
import { steamLoginUrl } from "@/lib/config";
import { getFaqs, getMyTickets, getSupportViewer, readTicketFilters, type SearchParams } from "@/lib/support";
import { MAX_ACTIVE_TICKETS } from "@/lib/supportEnums";
import type { FaqWithPeople } from "@/lib/types";
import styles from "@/components/Support/Support.module.scss";

export default function SupportPage({ searchParams }: PageProps<"/support">) {
  return (
    // The session, tickets and FAQs are request-time, so they stream in behind the boundary.
    <Suspense>
      <Support searchParams={searchParams} />
    </Suspense>
  );
}

/** /support: the user's on-going and finished tickets, plus a few FAQs. */
async function Support({ searchParams }: { searchParams: SearchParams }) {
  const [viewer, filters] = await Promise.all([getSupportViewer(), readTicketFilters(searchParams)]);

  const header = (
    <header className={styles.header}>
      <h1>Support</h1>
      <p>Get help from the community&apos;s administrators, or browse the frequently asked questions.</p>
    </header>
  );

  if (!viewer.userId) {
    return (
      <div className={styles.page}>
        {header}
        <SupportNav support={false} active="mine" />
        <p className={styles.notice}>
          {/* Plain anchor: a full navigation to the backend, which redirects to Steam. */}
          <a href={steamLoginUrl}>Log in with Steam</a> to open a support ticket or see your existing ones.
        </p>
        <FaqSection faqs={await getFaqs(undefined, 3)} />
      </div>
    );
  }

  const [active, done, faqList] = await Promise.all([
    getMyTickets(filters, "active"),
    getMyTickets(filters, "done"),
    getFaqs(undefined, 3),
  ]);

  return (
    <div className={styles.page}>
      <div className={styles.toolbar}>
        {header}
        <Link className={styles.primary} href="/support/new">
          New ticket
        </Link>
      </div>
      <SupportNav support={viewer.support} active="mine" />
      {viewer.banned && (
        <p className={styles.notice} role="status">
          Your account is banned from the rest of the site. You can still open and reply to support tickets, for
          example to appeal the ban.
        </p>
      )}
      <TicketFilterForm filters={filters} action="/support" />
      <section className={styles.section} aria-labelledby="active-heading">
        <div className={styles.sectionHead}>
          <h2 id="active-heading">On-going tickets</h2>
          <span>
            {active.length} of {MAX_ACTIVE_TICKETS} at once
          </span>
        </div>
        <TicketRows tickets={active} empty="No tickets in progress." />
      </section>
      <section className={styles.section} aria-labelledby="done-heading">
        <div className={styles.sectionHead}>
          <h2 id="done-heading">Resolved and closed</h2>
        </div>
        <TicketRows tickets={done} empty="No tickets have been addressed yet." />
      </section>
      <FaqSection faqs={faqList} />
    </div>
  );
}

function FaqSection({ faqs }: { faqs: FaqWithPeople[] }) {
  return (
    <section className={styles.section} aria-labelledby="faq-heading">
      <div className={styles.sectionHead}>
        <h2 id="faq-heading">Frequently asked questions</h2>
        <Link href="/support/faq">All FAQs</Link>
      </div>
      <FaqGrid faqs={faqs} empty="No FAQs have been written yet." />
    </section>
  );
}
