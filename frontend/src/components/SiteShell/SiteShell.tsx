import { Suspense, type ReactNode } from "react";
import { DemoBanner } from "@/components/DemoBanner";
import { SideNav } from "@/components/SideNav";

/**
 * The website frame: side navigation plus content. Used by the `(site)` layout, and by the
 * root-level error pages (401/403/404), which render outside that layout.
 */
export function SiteShell({ children }: { children: ReactNode }) {
  return (
    <div className="shell">
      <SideNav />
      <main className="content">
        {/* Request-time, so it streams in behind the boundary. */}
        <Suspense>
          <DemoBanner />
        </Suspense>
        {children}
      </main>
    </div>
  );
}
