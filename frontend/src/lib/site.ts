import "server-only";
import { headers } from "next/headers";

/** This site's origin as the browser sees it, e.g. `https://example.com`. Request-time. */
export async function siteOrigin(): Promise<string> {
  const h = await headers();
  const host = h.get("x-forwarded-host") ?? h.get("host") ?? "localhost:3000";
  const proto = h.get("x-forwarded-proto") ?? (host.startsWith("localhost") ? "http" : "https");
  return `${proto}://${host}`;
}

/** The `sv_loadingurl` line for `server.cfg`. Garry's Mod fills in `%s` (SteamID64) and `%m` (map). */
export function loadingUrlLine(origin: string, serverId: number | string): string {
  return `sv_loadingurl "${origin}/loading-screen?server=${serverId}&steamid=%s&mapname=%m"`;
}
