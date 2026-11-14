import { Suspense } from "react";
import type { Metadata, ResolvingMetadata } from "next";
import { notFound } from "next/navigation";
import { FaqView } from "@/components/Support";
import { pageMetadata } from "@/lib/pageMetadata";
import { getFaq, getSupportViewer } from "@/lib/support";

export async function generateMetadata(
  { params }: PageProps<"/support/faq/[faq_id]">,
  parent: ResolvingMetadata,
): Promise<Metadata> {
  const faq = await getFaq((await params).faq_id).catch(() => null);
  return faq ? pageMetadata(parent, faq.title, faq.subject) : {};
}

export default function FaqPage({ params }: PageProps<"/support/faq/[faq_id]">) {
  return (
    // Params, the FAQ and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {params.then(({ faq_id }) => (
        <Faq id={faq_id} />
      ))}
    </Suspense>
  );
}

/** /support/faq/{id}: the FAQ in full. Public; the Support Team can edit it. */
async function Faq({ id }: { id: string }) {
  const [faq, viewer] = await Promise.all([getFaq(id), getSupportViewer()]);
  if (!faq) notFound();
  return <FaqView faq={faq} support={viewer.support} />;
}
