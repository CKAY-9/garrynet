import type { LoadingScreenPlayer } from "@/lib/types";
import type { LoadingScreenServer } from "./LoadingScreen";

/** Fake server details for loading screen previews. */
export const previewServer: LoadingScreenServer = {
  name: "GarryNet DarkRP",
  gamemode: "darkrp",
  map: "rp_downtown_v4c_v2",
  icon_url: null,
  color: "#3b82f6",
};

const DAY = 24 * 60 * 60 * 1000;

/**
 * A fake returning player for loading screen previews. Dates are relative to now, so
 * "last logon" always reads "2 days ago". Build it on the server and pass it down, so the
 * server and client render the same dates.
 */
export function previewPlayer(): LoadingScreenPlayer {
  const now = Date.now();
  return {
    steam_id: "76561190000000000",
    name: "MelonEnjoyer",
    avatar_url: null,
    user_id: null,
    record: {
      id: 0,
      server_id: 0,
      steam_id: "76561190000000000",
      user_id: null,
      ingame_name: "MelonEnjoyer",
      last_logon: new Date(now - 2 * DAY).toISOString(),
      first_joined: new Date(now - 142 * DAY).toISOString(),
      ingame_usergroup: "vip",
      stats: { darkrp_money: 48250, darkrp_job: "Gun Dealer", darkrp_arrests: 3 },
      time_played: 7385,
    },
  };
}
