import { Suspense } from "react";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import {
  GMOD_BRIDGE_SCRIPT,
  STREAM_REVEAL_SCRIPT,
  LoadingScreen,
  LoadingScreenFrame,
  previewPlayer,
  previewServer,
  type LoadingScreenServer,
} from "@/components/LoadingScreen";
import {
  getLoadingScreenConfig,
  getLoadingScreenData,
} from "@/lib/loadingScreen";
import type { LoadingScreenData } from "@/lib/types";

export const metadata: Metadata = {
  title: "Loading...",
  robots: { index: false },
};

/**
 * The full-screen loading screen, outside the website layout. Servers use it as their
 * loading URL:
 *
 *     sv_loadingurl "https://<site>/loading-screen?server=<SERVER_ID>&steamid=%s&mapname=%m"
 *
 * Garry's Mod fills in `%s` (the player's SteamID64) and `%m` (the map). Opened without
 * any parameters, it previews the configured style with a fake player and server.
 */
export default function LoadingScreenPage({
  searchParams,
}: PageProps<"/loading-screen">) {
  return (
    <LoadingScreenFrame fullscreen label="Loading screen">
      {/* In the static shell, so it runs before Garry's Mod starts calling in. */}
      <script dangerouslySetInnerHTML={{ __html: GMOD_BRIDGE_SCRIPT }} />
      {/* Never flash white, even if the stylesheet is slow or unsupported. */}
      <style>
        {
          // Streamed content arrives in a `hidden` block that a script moves into place. If
          // scripts don't run in Garry's Mod, show that block where it is, filling the screen.
          'html,body{background:#000}div[id^="S:"]{display:block!important;position:fixed;top:0;right:0;bottom:0;left:0}'
        }
      </style>
      <script dangerouslySetInnerHTML={{ __html: STREAM_REVEAL_SCRIPT }} />
      {/* The query and the data are request-time, so they stream in behind the boundary. */}
      <Suspense>
        {searchParams.then((query) => {
          const serverId = single(query.server);
          const steamId = single(query.steamid);
          const map = single(query.mapname);
          return serverId || steamId || map ? (
            <InGame serverId={serverId} steamId={steamId} map={map} />
          ) : (
            <Preview />
          );
        })}
      </Suspense>
    </LoadingScreenFrame>
  );
}

/** What players see while joining a server. */
async function InGame({
  serverId,
  steamId,
  map,
}: {
  serverId: string | null;
  steamId: string | null;
  map: string | null;
}) {
  const fallback = { ...FALLBACK_SERVER, map: map ?? "" };
  // Players are mid-connect, so anything short of a wrong server id still shows a
  // working loading screen rather than an error page.
  if (!serverId) {
    return (
      <LoadingScreen
        style={await styleOrDefault()}
        server={fallback}
        player={null}
        mode="live"
      />
    );
  }

  let data: LoadingScreenData | null;
  try {
    data = await getLoadingScreenData(serverId, steamId);
  } catch (err) {
    console.error(
      "Failed to load loading screen data:",
      err instanceof Error ? err.message : err,
    );
    return (
      <LoadingScreen
        style="sleek"
        server={fallback}
        player={null}
        mode="live"
      />
    );
  }
  // A wrong server id in `sv_loadingurl`; the 404 makes the mistake obvious.
  if (!data) notFound();

  // Garry's Mod knows the map before the server's next heartbeat does.
  const server = { ...data.server, map: map || data.server.map };
  return (
    <LoadingScreen
      style={data.style}
      server={server}
      player={data.player}
      mode="live"
    />
  );
}

/** The configured style with a made-up player and server, and a looping fake load. */
async function Preview() {
  const { style } = await getLoadingScreenConfig();
  return (
    <LoadingScreen
      style={style}
      server={previewServer}
      player={previewPlayer()}
      mode="demo"
    />
  );
}

async function styleOrDefault() {
  try {
    return (await getLoadingScreenConfig()).style;
  } catch {
    return "sleek" as const;
  }
}

const FALLBACK_SERVER: LoadingScreenServer = {
  name: "Joining server",
  gamemode: "",
  map: "",
  icon_url: null,
  color: "#3b82f6",
};

function single(value: string | string[] | undefined): string | null {
  const v = Array.isArray(value) ? value[0] : value;
  return v?.trim() || null;
}
