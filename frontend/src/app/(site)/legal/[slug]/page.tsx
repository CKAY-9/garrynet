import { Suspense } from "react";
import { notFound } from "next/navigation";
import { Markdown } from "@/components/Markdown";
import { getLegalDocument } from "@/lib/legal";
import { dateFormat } from "@/lib/time";
import styles from "./legal.module.scss";

export default function LegalPage({ params }: PageProps<"/legal/[slug]">) {
  return (
    // Params and the document are request-time, so they stream in behind the boundary.
    <Suspense>
      {params.then(({ slug }) => (
        <LegalDocumentView slug={slug} />
      ))}
    </Suspense>
  );
}

/** /legal/{slug}: the terms, privacy policy or refund policy. Public. */
async function LegalDocumentView({ slug }: { slug: string }) {
  const document = await getLegalDocument(slug);
  if (!document) notFound();
  return (
    <article className={styles.page}>
      <header className={styles.header}>
        <h1>{document.title}</h1>
        {document.last_updated && (
          <p className={styles.updated}>Last updated {dateFormat.format(new Date(document.last_updated))}</p>
        )}
      </header>
      <Markdown>{document.body}</Markdown>
    </article>
  );
}
