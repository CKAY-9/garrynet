import { Suspense } from "react";
import { notFound, redirect } from "next/navigation";
import { getForumThread, positiveParam } from "@/lib/forum";

/**
 * /forum/threads/{id}: forum notifications link here (docs/NOTIFICATIONS.md) because they
 * only know the thread. Sends the user to the thread's real page, which needs the category.
 * `?post={id}` (a reply notification) carries over, scrolled to that post.
 */
export default function ThreadByIdPage({ params, searchParams }: PageProps<"/forum/threads/[thread_id]">) {
  return (
    // Params and the thread are request-time, so the lookup runs behind the boundary.
    <Suspense>
      {Promise.all([params, searchParams]).then(([{ thread_id }, { post }]) => (
        <ThreadRedirect threadId={thread_id} post={positiveParam(post)} />
      ))}
    </Suspense>
  );
}

async function ThreadRedirect({ threadId, post }: { threadId: string; post: number | undefined }): Promise<never> {
  // Only the category is needed, so the first page will do.
  const data = await getForumThread(threadId);
  if (!data) notFound();
  redirect(`/forum/${data.category.id}/${data.thread.id}${post ? `?post=${post}#post-${post}` : ""}`);
}
