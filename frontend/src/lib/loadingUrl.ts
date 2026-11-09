/** The `sv_loadingurl` line for `server.cfg`. Garry's Mod fills in `%s` (SteamID64) and `%m` (map). */
export function loadingUrlLine(origin: string, serverId: number | string): string {
  return `sv_loadingurl "${origin}/loading-screen?server=${serverId}&steamid=%s&mapname=%m"`;
}
