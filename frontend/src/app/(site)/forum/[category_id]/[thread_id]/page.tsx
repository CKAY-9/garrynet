import { Suspense } from "react";
import { notFound, redirect } from "next/navigation";
import { ForumThreadPage } from "@/components/Forum";
import { getForumThread, getPostingAccess } from "@/lib/forum";

export default function ThreadPage({ params }: PageProps<"/forum/[category_id]/[thread_id]">) {
  return (
    // Params, the thread and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {params.then(({ category_id, thread_id }) => (
        <Thread categoryId={category_id} threadId={thread_id} />
      ))}
    </Suspense>
  );
}

async function Thread({ categoryId, threadId }: { categoryId: string; threadId: string }) {
  const [data, access] = await Promise.all([getForumThread(threadId), getPostingAccess()]);
  if (!data) notFound();
  // Threads are looked up by id alone; send stale or mistyped category ids to the real one.
  if (String(data.category.id) !== categoryId) redirect(`/forum/${data.category.id}/${data.thread.id}`);
  return <ForumThreadPage {...data} access={access} />;
}
