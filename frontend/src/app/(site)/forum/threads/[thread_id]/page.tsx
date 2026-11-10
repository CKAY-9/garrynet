import { Suspense } from "react";
import { notFound, redirect } from "next/navigation";
import { getForumThread } from "@/lib/forum";

/**
 * /forum/threads/{id}: forum notifications link here (docs/NOTIFICATIONS.md) because they
 * only know the thread. Sends the user to the thread's real page, which needs the category.
 */
export default function ThreadByIdPage({ params }: PageProps<"/forum/threads/[thread_id]">) {
  return (
    // Params and the thread are request-time, so the lookup runs behind the boundary.
    <Suspense>{params.then(({ thread_id }) => <ThreadRedirect threadId={thread_id} />)}</Suspense>
  );
}

async function ThreadRedirect({ threadId }: { threadId: string }): Promise<never> {
  const data = await getForumThread(threadId);
  if (!data) notFound();
  redirect(`/forum/${data.category.id}/${data.thread.id}`);
}
