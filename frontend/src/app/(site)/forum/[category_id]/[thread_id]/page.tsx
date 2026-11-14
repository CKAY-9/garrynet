import { Suspense } from "react";
import type { Metadata, ResolvingMetadata } from "next";
import { notFound, redirect } from "next/navigation";
import { ForumThreadPage } from "@/components/Forum";
import { getForumThread, getForumViewer } from "@/lib/forum";
import { pageMetadata } from "@/lib/pageMetadata";
import { plainText } from "@/lib/text";

export async function generateMetadata(
  { params }: PageProps<"/forum/[category_id]/[thread_id]">,
  parent: ResolvingMetadata,
): Promise<Metadata> {
  const data = await getForumThread((await params).thread_id).catch(() => null);
  if (!data) return {};
  const first = data.posts.find((post) => post.id === data.thread.thread_post_id);
  return pageMetadata(parent, data.thread.title, first && plainText(first.content));
}

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
  const [data, viewer] = await Promise.all([getForumThread(threadId), getForumViewer()]);
  if (!data) notFound();
  // Threads are looked up by id alone; send stale or mistyped category ids to the real one.
  if (String(data.category.id) !== categoryId) redirect(`/forum/${data.category.id}/${data.thread.id}`);
  return <ForumThreadPage {...data} viewer={viewer} />;
}
