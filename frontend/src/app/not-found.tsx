import Link from "next/link";
import { SiteShell } from "@/components/SiteShell";
import { StatusPage } from "@/components/StatusPage";

// Rendered in place of the `(site)` layout, so it brings its own navigation.
export default function NotFound() {
  return (
    <SiteShell>
      <StatusPage code={404} title="Page not found">
        <p>There&apos;s nothing here.</p>
        <Link href="/">Return home</Link>
      </StatusPage>
    </SiteShell>
  );
}
