import "server-only";
import { connection } from "next/server";
import { authHeaders } from "./auth";
import { backend } from "./backend";
import type { AdminStats } from "./types";

/**
 * Dashboard statistics. The backend requires Admin Access, so call `requirePermission`
 * first. Request-time, so render anything that uses this inside a `<Suspense>` boundary.
 */
export async function getAdminStats(): Promise<AdminStats> {
  // Axios reads the clock, which prerendering forbids.
  await connection();
  const { data } = await backend.get<AdminStats>("/admin/stats", { headers: await authHeaders() });
  return data;
}
