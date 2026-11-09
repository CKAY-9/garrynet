import type { Metadata } from "next";
import { SideNav } from "@/components/SideNav";
import "./globals.scss";

export const metadata: Metadata = {
  title: "GarryNet",
  description: "An all-in-one web solution for Garry's Mod communities.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <body>
        <div className="shell">
          <SideNav />
          <main className="content">{children}</main>
        </div>
      </body>
    </html>
  );
}
