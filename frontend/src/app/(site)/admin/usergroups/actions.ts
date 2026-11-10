"use server";

import { refresh } from "next/cache";
import { redirect } from "next/navigation";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { Usergroup, UsergroupDetails } from "@/lib/types";

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
