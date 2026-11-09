import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { backend } from "./backend";
import type { UserProfile } from "./types";

/**
 * The public profile of the user with GarryNet id `id`, or `null` if there is no such
 * user. Request-time (status and last seen change constantly), so render anything that
 * uses this inside a `<Suspense>` boundary.
 */
export async function getUserProfile(id: string): Promise<UserProfile | null> {
  if (!/^\d+$/.test(id)) return null;

  // Axios reads the clock, which prerendering forbids.
  await connection();

  try {
    const { data } = await backend.get<UserProfile>(`/users/${id}`);
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}
