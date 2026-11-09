import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { backend } from "./backend";
import type { LoadingScreenConfig, LoadingScreenData } from "./types";

/**
 * The loading screen settings. Request-time, so render anything that uses this inside a
 * `<Suspense>` boundary.
 */
export async function getLoadingScreenConfig(): Promise<LoadingScreenConfig> {
  // Axios reads the clock, which prerendering forbids.
  await connection();
  const { data } = await backend.get<LoadingScreenConfig>("/loading-screen");
  return data;
}

/**
 * What server `serverId`'s loading screen shows to the player with SteamID64 `steamId`,
 * or `null` if there is no such server. Request-time, so render anything that uses this
 * inside a `<Suspense>` boundary.
 */
export async function getLoadingScreenData(
  serverId: string,
  steamId: string | null,
): Promise<LoadingScreenData | null> {
  if (!/^\d+$/.test(serverId)) return null;

  await connection();

  try {
    const { data } = await backend.get<LoadingScreenData>(`/servers/${serverId}/loading-screen`, {
      params: steamId ? { steam_id: steamId } : undefined,
    });
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}
