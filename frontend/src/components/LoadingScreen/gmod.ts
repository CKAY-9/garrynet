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

/**
 * React reveals streamed (`<Suspense>`) content with a script that waits on
 * `requestAnimationFrame` and timers, which Garry's Mod's loading panel doesn't reliably
 * run, leaving a blank screen. This ES5 fallback does the same swap from a plain
 * interval once the document has been parsed. React's own swap is then a no-op.
 */
export const STREAM_REVEAL_SCRIPT = `(function(){function r(){var t=document.querySelectorAll('template[id^="B:"]');for(var i=0;i<t.length;i++){var e=t[i],s=document.getElementById("S:"+e.id.slice(2)),a=e.previousSibling,z=e.nextSibling;if(!s||!a||!z||a.nodeType!==8||z.nodeType!==8)continue;var p=e.parentNode;while(a.nextSibling&&a.nextSibling!==z)p.removeChild(a.nextSibling);while(s.firstChild)p.insertBefore(s.firstChild,z);a.data="$";s.parentNode.removeChild(s)}}var n=0,h=setInterval(function(){if(document.readyState!=="loading")r();if(++n>200)clearInterval(h)},100)})();`;
