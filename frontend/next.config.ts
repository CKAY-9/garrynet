import type { NextConfig } from "next";

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
};

export default nextConfig;
