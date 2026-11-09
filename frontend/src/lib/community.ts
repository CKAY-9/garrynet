import "server-only";
import { cache } from "react";
import { connection } from "next/server";
import { backend } from "./backend";
import type { Community } from "./types";

/**
 * The community, or `null` if the backend can't be reached (pages still render, with
 * GarryNet defaults). Deduplicated per request, so the navigation and the page can both
 * call it. Request-time, so render anything that uses this inside a `<Suspense>` boundary.
 */
export const getCommunity = cache(async (): Promise<Community | null> => {
  // Axios reads the clock, which prerendering forbids.
  await connection();
  try {
    const { data } = await backend.get<Community>("/community");
    return data;
  } catch (err) {
    console.error("Failed to fetch community:", err instanceof Error ? err.message : err);
    return null;
  }
});

/** Shown when the community can't be loaded. */
export const FALLBACK_COMMUNITY_NAME = "GarryNet";

/** A link's host without `www.`, e.g. `discord.gg`, to label external links. */
export function linkLabel(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}
