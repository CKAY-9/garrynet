import type { Metadata } from "next";
import { themeScript } from "@/lib/theme";
import "./globals.scss";

export const metadata: Metadata = {
  title: "GarryNet",
  description: "An all-in-one web solution for Garry's Mod communities.",
};

/**
 * Bare document shell. Website pages get their navigation from `(site)/layout.tsx`; the
 * in-game loading screen (`/loading-screen/[server_id]`) renders full-screen without it.
 */
export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    // The theme script sets `data-theme` before React hydrates, so the attribute won't match.
    <html lang="en" suppressHydrationWarning>
      <head>
        {/* Applies the stored theme before the first paint, so it never flashes the other one. */}
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body>{children}</body>
    </html>
  );
}
