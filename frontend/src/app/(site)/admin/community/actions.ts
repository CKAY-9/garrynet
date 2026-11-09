"use server";

import axios from "axios";
import { refresh } from "next/cache";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { Community, CommunityDetails } from "@/lib/types";

export type SaveCommunityResult =
  | { ok: true; community: Community }
  | { ok: false; message: string };

/** Saves the community details. The backend validates them and checks the permissions. */
export async function saveCommunity(details: CommunityDetails): Promise<SaveCommunityResult> {
  try {
    const { data } = await backend.put<Community>("/community", details, { headers: await authHeaders() });
    // The navigation shows the community's name and logo on every page.
    refresh();
    return { ok: true, community: data };
  } catch (err) {
    const status = axios.isAxiosError(err) ? err.response?.status : undefined;
    if (status === 400) {
      const error = axios.isAxiosError(err) ? err.response?.data?.error : undefined;
      return { ok: false, message: typeof error === "string" ? capitalize(error) + "." : "Check the details and try again." };
    }
    if (status === 401) return { ok: false, message: "Your session has expired. Log in again to save." };
    if (status === 403) return { ok: false, message: "You don't have permission to edit the community." };
    // Log only the message: the axios error carries the request config, including the token.
    console.error("Failed to save community:", err instanceof Error ? err.message : err);
    return { ok: false, message: "Couldn't save the community. Try again." };
  }
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
