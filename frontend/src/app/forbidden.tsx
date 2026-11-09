import Link from "next/link";
import { SiteShell } from "@/components/SiteShell";
import { StatusPage } from "@/components/StatusPage";

// Rendered in place of the `(site)` layout, so it brings its own navigation.
export default function Forbidden() {
  return (
    <SiteShell>
      <StatusPage code={403} title="Access denied">
        <p>Your usergroups don&apos;t have permission to view this page.</p>
        <Link href="/">Return home</Link>
      </StatusPage>
    </SiteShell>
  );
}
