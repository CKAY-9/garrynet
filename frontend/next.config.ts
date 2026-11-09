import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  /* config options here */
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
};

export default nextConfig;
