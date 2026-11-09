import { Suspense } from "react";
import Form from "next/form";
import Link from "next/link";
import { FaqGrid, SupportNav } from "@/components/Support";
import { getFaqs, getSupportViewer, readFaqCategory, type SearchParams } from "@/lib/support";
import { faqCategories, faqCategoryLabels } from "@/lib/supportEnums";
import styles from "@/components/Support/Support.module.scss";

export default function FaqListPage({ searchParams }: PageProps<"/support/faq">) {
  return (
    // The session and FAQs are request-time, so they stream in behind the boundary.
    <Suspense>
      <Faqs searchParams={searchParams} />
    </Suspense>
  );
}

/** /support/faq: every FAQ grouped by category, or one category's. */
async function Faqs({ searchParams }: { searchParams: SearchParams }) {
  const [viewer, category] = await Promise.all([getSupportViewer(), readFaqCategory(searchParams)]);
  const faqs = await getFaqs(category);
  const groups = faqCategories
    .map((c) => [c, faqs.filter((f) => f.category === c)] as const)
    .filter(([, list]) => list.length > 0);

  return (
    <div className={styles.page}>
      <div className={styles.toolbar}>
        <header className={styles.header}>
          <h1>Frequently asked questions</h1>
          <p>Answers to common problems, complaints and questions.</p>
        </header>
        {viewer.support && (
          <Link className={styles.primary} href="/support/faq/new">
            New FAQ
          </Link>
        )}
      </div>
      <SupportNav support={viewer.support} active="faq" />
      <Form key={category} action="/support/faq" className={styles.filters} role="search">
        <label className={styles.field}>
          <span className={styles.label}>Category</span>
          <select className={styles.input} name="category" defaultValue={category}>
            <option value="">All categories</option>
            {faqCategories.map((c) => (
              <option key={c} value={c}>
                {faqCategoryLabels[c]}
              </option>
            ))}
          </select>
        </label>
        <div className={styles.actions}>
          <button className={styles.primary} type="submit">
            Apply
          </button>
          <Link className={styles.clear} href="/support/faq">
            Clear
          </Link>
        </div>
      </Form>
      {groups.map(([c, list]) => (
        <section key={c} className={styles.section} aria-labelledby={`faq-${c}`}>
          <div className={styles.sectionHead}>
            <h2 id={`faq-${c}`}>{faqCategoryLabels[c]}</h2>
            <span>
              {list.length} {list.length === 1 ? "FAQ" : "FAQs"}
            </span>
          </div>
          <FaqGrid faqs={list} empty="" />
        </section>
      ))}
      {faqs.length === 0 && (
        <p className={styles.notice}>
          {category ? `No FAQs in ${faqCategoryLabels[category]} yet.` : "No FAQs have been written yet."}
        </p>
      )}
    </div>
  );
}
