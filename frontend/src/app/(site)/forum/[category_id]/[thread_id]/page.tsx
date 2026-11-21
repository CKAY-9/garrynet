import { Suspense } from "react";
import type { Metadata, ResolvingMetadata } from "next";
import { notFound, redirect } from "next/navigation";
import { ForumThreadPage } from "@/components/Forum";
import { getForumThread, getForumViewer, positiveParam } from "@/lib/forum";
import { pageMetadata } from "@/lib/pageMetadata";
import { plainText } from "@/lib/text";

type Props = PageProps<"/forum/[category_id]/[thread_id]">;

/** Which page of the thread the URL asks for: `?page=`, or `?post=` for the page holding that post. */
async function readPage(searchParams: Props["searchParams"]): Promise<{ page: number; post?: number }> {
  const { page, post } = await searchParams;
  return { page: positiveParam(page) ?? 1, post: positiveParam(post) };
}

export async function generateMetadata({ params, searchParams }: Props, parent: ResolvingMetadata): Promise<Metadata> {
  // The same fetch as the page's, so it's made once. Only page 1 has the first post, so
  // later pages go without a description rather than making a second request for it.
  const [{ thread_id }, { page, post }] = await Promise.all([params, readPage(searchParams)]);
  const data = await getForumThread(thread_id, page, post).catch(() => null);
  if (!data) return {};
  const first = data.posts.find((p) => p.id === data.thread.thread_post_id);
  return pageMetadata(parent, data.thread.title, first && plainText(first.content));
}

export default function ThreadPage({ params, searchParams }: Props) {
  return (
    // Params, the thread and the session are request-time, so they stream in behind the boundary.
    <Suspense>
      {Promise.all([params, readPage(searchParams)]).then(([{ category_id, thread_id }, { page, post }]) => (
        <Thread categoryId={category_id} threadId={thread_id} page={page} post={post} />
      ))}
    </Suspense>
  );
}

async function Thread({
  categoryId,
  threadId,
  page,
  post,
}: {
  categoryId: string;
  threadId: string;
  page: number;
  post: number | undefined;
}) {
  const [data, viewer] = await Promise.all([getForumThread(threadId, page, post), getForumViewer()]);
  if (!data) notFound();
  // Threads are looked up by id alone; send stale or mistyped category ids to the real one.
  if (String(data.category.id) !== categoryId) {
    redirect(`/forum/${data.category.id}/${data.thread.id}${post ? `?post=${post}#post-${post}` : page > 1 ? `?page=${page}` : ""}`);
  }
  return <ForumThreadPage {...data} viewer={viewer} />;
}
