"use server";

import { refresh } from "next/cache";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { CurrentUser, ProfileSettings } from "@/lib/types";

/**
 * Saves the current user's bio and profile privacy (docs/USERS.md "Profile Settings"). The
 * backend validates them and refuses banned users.
 */
export async function saveProfileSettings(settings: ProfileSettings): Promise<ActionResult<CurrentUser>> {
  return runAction(
    "save your settings",
    async () => {
      const { data } = await backend.patch<CurrentUser>("/users/me", settings, { headers: await authHeaders() });
      refresh();
      return data;
    },
    { 403: "Banned users can't change their profile." },
  );
}
