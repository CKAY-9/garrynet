import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { backend } from "./backend";
import type { UsergroupBadge, UserProfile, UserSearchFilters, UserSearchResult } from "./types";

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

/**
 * Users matching `filters`; random users when there are none. Throws on a 400 (e.g. an
 * id that isn't a number) as `UserSearchError`. Request-time, like `getUserProfile`.
 */
export async function searchUsers(filters: UserSearchFilters): Promise<UserSearchResult[]> {
  await connection();

  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    const trimmed = value?.trim();
    if (trimmed) params.set(key, trimmed);
  }

  try {
    // URLSearchParams encodes `+` in `reputation+` as `%2B`, so it survives the trip.
    const { data } = await backend.get<UserSearchResult[]>(`/users/search?${params}`);
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 400) {
      throw new UserSearchError(err.response.data?.error ?? "Invalid search.");
    }
    throw err;
  }
}

/** The backend rejected a search's filters; `message` says why. */
export class UserSearchError extends Error {}

/** Every usergroup's display details, e.g. for a filter dropdown. Request-time. */
export async function getUsergroups(): Promise<UsergroupBadge[]> {
  await connection();
  const { data } = await backend.get<UsergroupBadge[]>("/usergroups");
  return data;
}
