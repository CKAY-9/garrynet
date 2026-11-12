import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { backend } from "./backend";
import { authHeaders } from "./auth";
import type {
  AuthoredFilters,
  AuthoredPage,
  Punishment,
  Usergroup,
  UsergroupBadge,
  UsergroupMember,
  UserProfile,
  UserSearchFilters,
  UserSearchResult,
} from "./types";

/**
 * The public profile of the user with GarryNet id `id`, or `null` if there is no such
 * user. Sent as the current user, so a private profile's details come back for its owner
 * and staff. Request-time (status and last seen change constantly), so render anything
 * that uses this inside a `<Suspense>` boundary.
 */
export async function getUserProfile(id: string): Promise<UserProfile | null> {
  if (!/^\d+$/.test(id)) return null;

  // Axios reads the clock, which prerendering forbids.
  await connection();

  try {
    const { data } = await backend.get<UserProfile>(`/users/${id}`, { headers: await authHeaders() });
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

/**
 * The user's punishments, newest first. Staff (Admin Access and User Management) get the
 * whole history; users get their own punishments in effect. `null` for anyone else, or if
 * they couldn't be loaded (the profile still renders). Request-time.
 */
export async function getUserPunishments(id: number): Promise<Punishment[] | null> {
  await connection();
  try {
    const { data } = await backend.get<Punishment[]>(`/users/${id}/punishments`, { headers: await authHeaders() });
    return data;
  } catch (err) {
    const status = axios.isAxiosError(err) ? err.response?.status : undefined;
    if (status !== 401 && status !== 403) {
      // Log only the message: the axios error carries the request config, including the token.
      console.error(`Failed to load the punishments of user ${id}:`, err instanceof Error ? err.message : err);
    }
    return null;
  }
}

/** Threads and posts per page of a profile's activity. */
export const AUTHORED_PAGE_SIZE = 20;

/**
 * Page `page` (from 1) of the threads and posts user `id` wrote that match `filters`, or
 * `null` if the viewer may not see them (a private profile). Request-time.
 */
export async function getUserPosts(id: number, filters: AuthoredFilters, page: number): Promise<AuthoredPage | null> {
  await connection();

  const params = new URLSearchParams({
    limit: String(AUTHORED_PAGE_SIZE),
    offset: String((page - 1) * AUTHORED_PAGE_SIZE),
  });
  for (const [key, value] of Object.entries(filters)) {
    if (value.trim()) params.set(key, value.trim());
  }

  try {
    const { data } = await backend.get<AuthoredPage>(`/users/${id}/posts?${params}`, { headers: await authHeaders() });
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 403) return null;
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

/**
 * Every usergroup with its members and permissions. The backend requires Admin Access and
 * Usergroup Management, so call `requirePermission` first. Request-time.
 */
export async function getAdminUsergroups(): Promise<Usergroup[]> {
  await connection();
  const { data } = await backend.get<Usergroup[]>("/admin/usergroups", { headers: await authHeaders() });
  return data;
}

/** One usergroup in full, or `null` if there is no such group. Same requirements as `getAdminUsergroups`. */
export async function getAdminUsergroup(id: string): Promise<Usergroup | null> {
  if (!/^\d+$/.test(id)) return null;
  await connection();
  try {
    const { data } = await backend.get<Usergroup>(`/admin/usergroups/${id}`, { headers: await authHeaders() });
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

/** A usergroup's members with names and avatars, by name. Same requirements as `getAdminUsergroups`. */
export async function getUsergroupMembers(id: number): Promise<UsergroupMember[]> {
  await connection();
  const { data } = await backend.get<UsergroupMember[]>(`/admin/usergroups/${id}/members`, {
    headers: await authHeaders(),
  });
  return data;
}
