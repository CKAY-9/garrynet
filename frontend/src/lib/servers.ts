import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { backend } from "./backend";
import type { Server } from "./types";

/** Minutes without a heartbeat before a linked server counts as offline. */
const ONLINE_WINDOW_MINUTES = 5;

/** Every registered server, oldest first. Request-time; render inside a `<Suspense>` boundary. */
export async function getServers(): Promise<Server[]> {
  // Axios reads the clock, which prerendering forbids.
  await connection();
  const { data } = await backend.get<Server[]>("/servers");
  return data;
}

/**
 * The server with GarryNet id `id`, or `null` if there is no such server. Request-time;
 * render inside a `<Suspense>` boundary.
 */
export async function getServer(id: string): Promise<Server | null> {
  if (!/^\d+$/.test(id)) return null;
  await connection();
  try {
    const { data } = await backend.get<Server>(`/servers/${id}`);
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

export type ServerStatus = "unlinked" | "waiting" | "online" | "offline";

/**
 * - unlinked: the addon hasn't connected with the access key yet
 * - waiting: linked, but no heartbeat yet
 * - online / offline: by whether the last heartbeat is recent
 */
export function serverStatus(server: Server, now = Date.now()): ServerStatus {
  if (!server.linked) return "unlinked";
  if (!server.last_heartbeat) return "waiting";
  const age = now - new Date(server.last_heartbeat).getTime();
  return age <= ONLINE_WINDOW_MINUTES * 60_000 ? "online" : "offline";
}
