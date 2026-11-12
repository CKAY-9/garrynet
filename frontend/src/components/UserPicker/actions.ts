"use server";

import { runAction, type ActionResult } from "@/lib/actionResult";
import { backend } from "@/lib/backend";
import type { UserRef, UserSearchResult } from "@/lib/types";

/** How many users the picker lists. */
const LIMIT = 8;

/**
 * Users whose name contains `query`, plus the user with that id if it's a number (listed
 * first; a leading `#` is ignored). At most 8. User search is public, so no login is needed.
 */
export async function findUsers(query: string): Promise<ActionResult<UserRef[]>> {
  const trimmed = query.trim();
  if (!trimmed) return { ok: true, data: [] };
  const id = trimmed.replace(/^#/, "");
  return runAction("search users", async () => {
    const search = (params: Record<string, string>) =>
      backend
        .get<UserSearchResult[]>(`/users/search?${new URLSearchParams({ ...params, limit: String(LIMIT) })}`)
        .then((res) => res.data);
    const [byId, byName] = await Promise.all([
      /^\d{1,18}$/.test(id) ? search({ id }) : Promise.resolve([]),
      search({ name: trimmed }),
    ]);
    const seen = new Set<number>();
    return [...byId, ...byName]
      .filter((u) => !seen.has(u.id) && seen.add(u.id))
      .slice(0, LIMIT)
      .map(({ id, username, avatar_url }) => ({ id, username, avatar_url }));
  });
}
