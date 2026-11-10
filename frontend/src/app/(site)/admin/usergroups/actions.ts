"use server";

import { refresh } from "next/cache";
import { redirect } from "next/navigation";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { Usergroup, UsergroupDetails, UsergroupMember, UserSearchResult } from "@/lib/types";

/*
 * Usergroup management (docs/PERMISSIONS.md). The backend checks Admin Access plus
 * Usergroup Management, that only permissions the editor holds are granted or revoked, and
 * validates everything; these just relay the result to the forms.
 */

const MESSAGES = {
  403: "You can only grant or revoke permissions you hold yourself.",
  409: "A usergroup with this name already exists.",
};

/** Creates a usergroup, then opens its page. */
export async function createUsergroup(details: UsergroupDetails): Promise<ActionResult<never>> {
  const result = await runAction(
    "create the usergroup",
    async () => {
      const { data } = await backend.post<Usergroup>("/admin/usergroups", details, { headers: await authHeaders() });
      return data;
    },
    MESSAGES,
  );
  if (!result.ok) return result;
  redirect(`/admin/usergroups/${result.data.id}`);
}

export async function updateUsergroup(id: number, details: UsergroupDetails): Promise<ActionResult<Usergroup>> {
  return runAction(
    "save the usergroup",
    async () => {
      const { data } = await backend.put<Usergroup>(`/admin/usergroups/${id}`, details, {
        headers: await authHeaders(),
      });
      refresh();
      return data;
    },
    MESSAGES,
  );
}

/** Deletes a usergroup, then returns to the list. */
export async function deleteUsergroup(id: number): Promise<ActionResult<never>> {
  const result = await runAction("delete the usergroup", async () => {
    await backend.delete(`/admin/usergroups/${id}`, { headers: await authHeaders() });
  });
  if (!result.ok) return result;
  redirect("/admin/usergroups");
}

const MEMBER_MESSAGES = {
  403: "You can only change the members of groups whose permissions you hold yourself.",
  409: "That user is already a member.",
};

/** A user found by `findUsersToAdd`. */
export type UserRef = Pick<UserSearchResult, "id" | "username" | "avatar_url">;

/** How many users the "Add user" search shows. */
const ADD_SEARCH_LIMIT = 10;

/**
 * Users whose name contains `query`, plus the user with that id if it's a number (listed
 * first). At most 10.
 */
export async function findUsersToAdd(query: string): Promise<ActionResult<UserRef[]>> {
  const trimmed = query.trim();
  if (!trimmed) return { ok: true, data: [] };
  return runAction("search users", async () => {
    const search = (params: Record<string, string>) =>
      backend
        .get<UserSearchResult[]>(`/users/search?${new URLSearchParams({ ...params, limit: String(ADD_SEARCH_LIMIT) })}`)
        .then((res) => res.data);
    const [byId, byName] = await Promise.all([
      /^\d{1,18}$/.test(trimmed) ? search({ id: trimmed }) : Promise.resolve([]),
      search({ name: trimmed }),
    ]);
    const seen = new Set<number>();
    return [...byId, ...byName]
      .filter((u) => !seen.has(u.id) && seen.add(u.id))
      .slice(0, ADD_SEARCH_LIMIT)
      .map(({ id, username, avatar_url }) => ({ id, username, avatar_url }));
  });
}

export async function addUsergroupMember(groupId: number, userId: number): Promise<ActionResult<UsergroupMember>> {
  return runAction(
    "add the user",
    async () => {
      const { data } = await backend.post<UsergroupMember>(
        `/admin/usergroups/${groupId}/members`,
        { user_id: userId },
        { headers: await authHeaders() },
      );
      refresh();
      return data;
    },
    MEMBER_MESSAGES,
  );
}

export async function removeUsergroupMember(groupId: number, userId: number): Promise<ActionResult<null>> {
  return runAction(
    "remove the user",
    async () => {
      await backend.delete(`/admin/usergroups/${groupId}/members/${userId}`, { headers: await authHeaders() });
      refresh();
      return null;
    },
    MEMBER_MESSAGES,
  );
}
