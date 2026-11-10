"use server";

import { refresh } from "next/cache";
import { redirect } from "next/navigation";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { ForumPost, ForumThread } from "@/lib/types";

/** A new thread's first post and flags. Starting a thread locked needs Thread Management. */
export type NewThread = {
  title: string;
  content: string;
  locked: boolean;
  allow_template: boolean;
  /** IDs of uploaded files to attach to the first post. */
  attachments: string[];
};

/** Starts a thread, then opens it. The backend validates it and checks the permissions. */
export async function createThread(categoryId: number, thread: NewThread): Promise<ActionResult<never>> {
  const result = await runAction("start a thread", async () => {
    const { data } = await backend.post<ForumThread>(
      `/forum/categories/${categoryId}/threads`,
      thread,
      { headers: await authHeaders() },
    );
    return data;
  });
  if (!result.ok) return result;
  redirect(`/forum/${result.data.category_id}/${result.data.id}`);
}

/** Replies to a thread, attaching the uploaded files `attachments`, and refreshes the page to show the new post. */
export async function replyToThread(
  threadId: number,
  content: string,
  attachments: string[] = [],
): Promise<ActionResult<ForumPost>> {
  return runAction("post the reply", async () => {
    const { data } = await backend.post<ForumPost>(
      `/forum/threads/${threadId}/posts`,
      { content, attachments },
      { headers: await authHeaders() },
    );
    refresh();
    return data;
  });
}

/** Thread fields to change; omitted ones stay as they are. Locking and stickying need Thread Management. */
export type ThreadChanges = { title?: string; locked?: boolean; sticky?: boolean; allow_template?: boolean };

/** Changes a thread and refreshes the page. */
export async function updateThread(threadId: number, changes: ThreadChanges): Promise<ActionResult<ForumThread>> {
  return runAction("change the thread", async () => {
    const { data } = await backend.patch<ForumThread>(`/forum/threads/${threadId}`, changes, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

/** Deletes a thread with all its posts, then opens its category. */
export async function deleteThread(threadId: number, categoryId: number): Promise<ActionResult<never>> {
  const result = await runAction("delete the thread", async () => {
    await backend.delete(`/forum/threads/${threadId}`, { headers: await authHeaders() });
  });
  if (!result.ok) return result;
  redirect(`/forum/${categoryId}`);
}

/** Replaces a post's content and refreshes the page. */
export async function editPost(postId: number, content: string): Promise<ActionResult<ForumPost>> {
  return runAction("edit the post", async () => {
    const { data } = await backend.patch<ForumPost>(`/forum/posts/${postId}`, { content }, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

/** Deletes a reply and refreshes the page. Delete a thread's first post with `deleteThread`. */
export async function deletePost(postId: number): Promise<ActionResult<void>> {
  return runAction("delete the post", async () => {
    await backend.delete(`/forum/posts/${postId}`, { headers: await authHeaders() });
    refresh();
  });
}
