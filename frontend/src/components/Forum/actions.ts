"use server";

import { refresh } from "next/cache";
import { redirect } from "next/navigation";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { ForumPost, ForumThread } from "@/lib/types";

/** Starts a thread, then opens it. The backend validates it and checks the permissions. */
export async function createThread(categoryId: number, title: string, content: string): Promise<ActionResult<never>> {
  const result = await runAction("start a thread", async () => {
    const { data } = await backend.post<ForumThread>(
      `/forum/categories/${categoryId}/threads`,
      { title, content },
      { headers: await authHeaders() },
    );
    return data;
  });
  if (!result.ok) return result;
  redirect(`/forum/${result.data.category_id}/${result.data.id}`);
}

/** Replies to a thread and refreshes the page to show the new post. */
export async function replyToThread(threadId: number, content: string): Promise<ActionResult<ForumPost>> {
  return runAction("post the reply", async () => {
    const { data } = await backend.post<ForumPost>(
      `/forum/threads/${threadId}/posts`,
      { content },
      { headers: await authHeaders() },
    );
    refresh();
    return data;
  });
}
