import { requireHttps } from "@/lib/secureUrl";

/**
 * Runs once when the server starts: refuses to start a production server whose backend URLs
 * would send login tokens over plain HTTP across the network. `API_URL` is read at runtime
 * (Docker Compose sets it); `NEXT_PUBLIC_API_URL` was checked at build time (next.config.ts),
 * and is checked again here for servers started without rebuilding.
 */
export function register() {
  requireHttps([
    ["API_URL", process.env.API_URL, "internal"],
    ["NEXT_PUBLIC_API_URL", process.env.NEXT_PUBLIC_API_URL, "public"],
  ]);
}
