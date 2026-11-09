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

/** Reads the session cookie; see `getCurrentUser`. */
export async function getPostingAccess(): Promise<PostingAccess> {
  const user = await getCurrentUser();
  if (!user) return "guest";
  return hasPermission(user.permissions, Permission.SiteAccess | Permission.UserPosts) ? "allowed" : "forbidden";
}
