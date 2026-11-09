import type { Metadata } from "next";
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
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
