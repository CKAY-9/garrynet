import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { getCurrentUser } from "./auth";
import { backend } from "./backend";
import { hasPermission, Permission } from "./permissions";
import type { ForumCategoryPage, ForumHome, ForumThreadPage } from "./types";

/*
 * Forum reads. All request-time (posts change constantly), so render anything that uses
 * these inside a `<Suspense>` boundary. Writes are server actions: posting in
 * `@/components/Forum/actions`, managing sections and categories in `admin/forum/actions`.
 */

/** Every section with its categories, plus forum statistics. */
export async function getForumHome(): Promise<ForumHome> {
  // Axios reads the clock, which prerendering forbids.
  await connection();
  const { data } = await backend.get<ForumHome>("/forum");
  return data;
}

/** The category with id `id` and its threads, or `null` if there is no such category. */
export async function getForumCategory(id: string): Promise<ForumCategoryPage | null> {
  return getOrNull<ForumCategoryPage>(id, (n) => `/forum/categories/${n}`);
}

/** The thread with id `id` and its posts, or `null` if there is no such thread. */
export async function getForumThread(id: string): Promise<ForumThreadPage | null> {
  return getOrNull<ForumThreadPage>(id, (n) => `/forum/threads/${n}`);
}

/**
 * The title and first post of thread `id` to start a new thread from, or `null` if there is
 * no such thread or it doesn't allow templating.
 */
export async function getForumTemplate(id: string | undefined): Promise<{ id: number; title: string; content: string } | null> {
  if (!id) return null;
  const data = await getForumThread(id);
  if (!data?.thread.allow_template) return null;
  const first = data.posts.find((p) => p.id === data.thread.thread_post_id);
  return first ? { id: data.thread.id, title: data.thread.title, content: first.content } : null;
}

async function getOrNull<T>(id: string, path: (id: string) => string): Promise<T | null> {
  if (!/^\d+$/.test(id)) return null;
  await connection();
  try {
    const { data } = await backend.get<T>(path(id));
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

/** Whether the current user may post: logged out, logged in without permission, or allowed. */
export type PostingAccess = "guest" | "forbidden" | "allowed";

/** What the current user can do on the forum. */
export type ForumViewer = {
  access: PostingAccess;
  /** `null` when logged out. */
  userId: number | null;
  /** Has Thread Management: can lock, sticky, edit and delete anyone's threads and posts. */
  moderator: boolean;
};

/** Reads the session cookie; see `getCurrentUser`. The backend enforces all of this too. */
export async function getForumViewer(): Promise<ForumViewer> {
  const user = await getCurrentUser();
  if (!user) return { access: "guest", userId: null, moderator: false };
  return {
    access: hasPermission(user.permissions, Permission.SiteAccess | Permission.UserPosts) ? "allowed" : "forbidden",
    userId: user.id,
    moderator: hasPermission(user.permissions, Permission.SiteAccess | Permission.ThreadManagement),
  };
}
