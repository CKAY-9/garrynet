import "server-only";
import { cache } from "react";
import { connection } from "next/server";
import { backend } from "./backend";
import type { DemoStatus } from "./types";

/**
 * Whether this is a demo deployment (docs/DEMO.md). Not a demo if the backend can't be
 * reached. Deduplicated per request. Request-time, so render anything that uses this inside
 * a `<Suspense>` boundary.
 */
export const getDemo = cache(async (): Promise<DemoStatus> => {
  // Axios reads the clock, which prerendering forbids.
  await connection();
  try {
    const { data } = await backend.get<DemoStatus>("/demo");
    return data;
  } catch (err) {
    console.error("Failed to fetch demo status:", err instanceof Error ? err.message : err);
    return { enabled: false, next_reset: null };
  }
});
