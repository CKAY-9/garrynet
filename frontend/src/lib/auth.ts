import "server-only";
import { cache } from "react";
import { cookies } from "next/headers";
import { forbidden, unauthorized } from "next/navigation";
import { connection } from "next/server";
import axios from "axios";
import { backend } from "./backend";
import { hasPermission } from "./permissions";
import type { CurrentUser } from "./types";

/** httpOnly cookie the backend sets after Steam login. Must match `AUTH_COOKIE` in the backend. */
export const AUTH_COOKIE = "garrynet_token";

/**
 * The user's JWT, read from the request cookies. Server-only: the cookie is httpOnly, so
 * client code can't see it. Reading cookies makes the caller request-time, so render
 * anything that uses this inside a `<Suspense>` boundary.
 */
export async function getToken(): Promise<string | null> {
  return (await cookies()).get(AUTH_COOKIE)?.value ?? null;
}

/** Headers that authenticate a backend request as the current user. */
export async function authHeaders(): Promise<Record<string, string>> {
  const token = await getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

/**
 * The logged-in user, or `null` if logged out or the token is invalid/expired.
 * Deduplicated per request, so any number of components can call it.
 */
export const getCurrentUser = cache(async (): Promise<CurrentUser | null> => {
  const headers = await authHeaders();
  if (!headers.Authorization) return null;

  // The lookup also marks the user as online, so it must only run for real requests, never
  // while prerendering (axios also reads the clock, which prerendering forbids).
  await connection();

  try {
    const { data } = await backend.get<CurrentUser>("/users/me", { headers });
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 401) return null;
    // Log only the message: the axios error carries the request config, including the token.
    console.error("Failed to fetch current user:", err instanceof Error ? err.message : err);
    return null;
  }
});

/**
 * The logged-in user, provided they hold every permission in `required`. Otherwise renders
 * the 401 page (logged out) or the 403 page (missing permission). Call it in each protected
 * page, not only its layout: layouts don't re-render on every navigation. The backend checks
 * permissions too, so this only decides what UI to show.
 */
export async function requirePermission(required: number): Promise<CurrentUser> {
  const user = await getCurrentUser();
  if (!user) unauthorized();
  if (!hasPermission(user.permissions, required)) forbidden();
  return user;
}
