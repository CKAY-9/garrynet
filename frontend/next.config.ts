import type { NextConfig } from "next";
import { requireHttps } from "./src/lib/secureUrl";

// `next build` inlines NEXT_PUBLIC_API_URL into the browser bundle, so a plain-HTTP backend
// outside localhost is refused here, before anything ships (see src/lib/secureUrl.ts).
requireHttps([["NEXT_PUBLIC_API_URL", process.env.NEXT_PUBLIC_API_URL, "public"]]);

const nextConfig: NextConfig = {
  // Self-contained server output for the Docker image.
  output: "standalone",
  cacheComponents: true,
  partialPrefetching: true,
  experimental: {
    // Enables `forbidden()` / `unauthorized()` for permission-gated pages (e.g. /admin).
    authInterrupts: true,
  },
  images: {
    // Steam avatars (avatars.steamstatic.com and its CDN mirrors).
    remotePatterns: [{ protocol: "https", hostname: "**.steamstatic.com" }],
  },
  allowedDevOrigins: ['loopback.gmod'],
  async headers() {
    // Production sites are served over HTTPS (behind Caddy, which adds this too): browsers
    // that have seen the site never use plain HTTP for it again, for a year. Browsers ignore
    // it over plain HTTP, e.g. `next start` on localhost. Not includeSubDomains: other services
    // may share the parent domain.
    if (process.env.NODE_ENV !== "production") return [];
    return [{ source: "/:path*", headers: [{ key: "Strict-Transport-Security", value: "max-age=31536000" }] }];
  },
};

export default nextConfig;
