import { Suspense } from "react";
import { notFound } from "next/navigation";
import { ForumCategoryPage } from "@/components/Forum";
import { getForumCategory, getPostingAccess } from "@/lib/forum";

export default function CategoryPage({ params }: PageProps<"/forum/[category_id]">) {
  return (
    // Params, the category and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {params.then(({ category_id }) => (
        <Category id={category_id} />
      ))}
    </Suspense>
  );
}

async function Category({ id }: { id: string }) {
  const [data, access] = await Promise.all([getForumCategory(id), getPostingAccess()]);
  if (!data) notFound();
  return <ForumCategoryPage {...data} access={access} />;
}
