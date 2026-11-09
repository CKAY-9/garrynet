/**
 * The global functions Garry's Mod calls on its loading screen while the player connects:
 *
 * - `GameDetails(serverName, serverUrl, mapName, maxPlayers, steamId, gamemode, volume, language)`
 * - `SetFilesTotal(total)`, `SetFilesNeeded(needed)`, `DownloadingFile(fileName)`
 * - `SetStatusChanged(status)`, e.g. "Sending client info..."
 */
export const GMOD_CALLBACKS = [
  "GameDetails",
  "SetFilesTotal",
  "SetFilesNeeded",
  "DownloadingFile",
  "SetStatusChanged",
] as const;

export type GmodCallback = (typeof GMOD_CALLBACKS)[number];

/** Window property holding calls made before React took over. */
export const GMOD_QUEUE = "__garrynetGmodQueue";

declare global {
  interface Window {
    GameDetails?: (...args: unknown[]) => void;
    SetFilesTotal?: (...args: unknown[]) => void;
    SetFilesNeeded?: (...args: unknown[]) => void;
    DownloadingFile?: (...args: unknown[]) => void;
    SetStatusChanged?: (...args: unknown[]) => void;
    [GMOD_QUEUE]?: [GmodCallback, unknown[]][];
  }
}

/**
 * Inline script that defines every callback as a stub that queues its call. Garry's Mod
 * can call them before the page has hydrated; `LoadingProgress` replays the queue.
 */
export const GMOD_BRIDGE_SCRIPT = `(function(){var q=window.${GMOD_QUEUE}=[];${JSON.stringify(
  GMOD_CALLBACKS,
)}.forEach(function(n){window[n]=function(){q.push([n,Array.prototype.slice.call(arguments)])}})})();`;
