import Link from "next/link";
import { StatusPage } from "@/components/StatusPage";

// Shown when a `(site)` page calls `forbidden()`. It renders inside the `(site)` layout, which
// already has the navigation. Without this file Next reuses `app/forbidden.tsx` here, whose
// own navigation would then appear inside the content area.
export default function Forbidden() {
  return (
    <StatusPage code={403} title="Access denied">
      <p>Your usergroups don&apos;t have permission to view this page.</p>
      <Link href="/">Return home</Link>
    </StatusPage>
  );
}
