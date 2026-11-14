import { Suspense } from "react";
import type { Metadata, ResolvingMetadata } from "next";
import { notFound } from "next/navigation";
import { ForumCategoryPage } from "@/components/Forum";
import { getForumCategory, getForumTemplate, getForumViewer } from "@/lib/forum";
import { pageMetadata } from "@/lib/pageMetadata";

export async function generateMetadata(
  { params }: PageProps<"/forum/[category_id]">,
  parent: ResolvingMetadata,
): Promise<Metadata> {
  const data = await getForumCategory((await params).category_id).catch(() => null);
  return data ? pageMetadata(parent, data.category.name, data.category.description) : {};
}

export default function CategoryPage({ params, searchParams }: PageProps<"/forum/[category_id]">) {
  return (
    // Params, the category and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {Promise.all([params, searchParams]).then(([{ category_id }, { template }]) => (
        <Category id={category_id} templateId={typeof template === "string" ? template : undefined} />
      ))}
    </Suspense>
  );
}

/** `?template={thread_id}` opens the new thread form filled in from that thread ("Use as template"). */
async function Category({ id, templateId }: { id: string; templateId: string | undefined }) {
  const [data, viewer, template] = await Promise.all([
    getForumCategory(id),
    getForumViewer(),
    getForumTemplate(templateId),
  ]);
  if (!data) notFound();
  return <ForumCategoryPage {...data} viewer={viewer} template={template ?? undefined} />;
}
