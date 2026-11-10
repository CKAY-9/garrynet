import { Suspense } from "react";
import { notFound } from "next/navigation";
import { ForumCategoryPage } from "@/components/Forum";
import { getForumCategory, getForumTemplate, getForumViewer } from "@/lib/forum";

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
