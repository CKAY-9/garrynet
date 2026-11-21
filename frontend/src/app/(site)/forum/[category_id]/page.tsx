import { Suspense } from "react";
import type { Metadata, ResolvingMetadata } from "next";
import { notFound } from "next/navigation";
import { ForumCategoryPage } from "@/components/Forum";
import { getForumCategory, getForumTemplate, getForumViewer, positiveParam } from "@/lib/forum";
import { pageMetadata } from "@/lib/pageMetadata";

export async function generateMetadata(
  { params, searchParams }: PageProps<"/forum/[category_id]">,
  parent: ResolvingMetadata,
): Promise<Metadata> {
  // The same fetch as the page's, so it's made once.
  const [{ category_id }, { page }] = await Promise.all([params, searchParams]);
  const data = await getForumCategory(category_id, positiveParam(page) ?? 1).catch(() => null);
  return data ? pageMetadata(parent, data.category.name, data.category.description) : {};
}

export default function CategoryPage({ params, searchParams }: PageProps<"/forum/[category_id]">) {
  return (
    // Params, the category and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {Promise.all([params, searchParams]).then(([{ category_id }, { template, page }]) => (
        <Category
          id={category_id}
          page={positiveParam(page) ?? 1}
          templateId={typeof template === "string" ? template : undefined}
        />
      ))}
    </Suspense>
  );
}

/**
 * `?page=` picks the page of threads. `?template={thread_id}` opens the new thread form filled
 * in from that thread ("Use as template").
 */
async function Category({ id, page, templateId }: { id: string; page: number; templateId: string | undefined }) {
  const [data, viewer, template] = await Promise.all([
    getForumCategory(id, page),
    getForumViewer(),
    getForumTemplate(templateId),
  ]);
  if (!data) notFound();
  return <ForumCategoryPage {...data} viewer={viewer} template={template ?? undefined} />;
}
