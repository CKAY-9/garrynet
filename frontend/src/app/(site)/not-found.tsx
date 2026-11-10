import Link from "next/link";
import { StatusPage } from "@/components/StatusPage";

// Shown when a `(site)` page calls `notFound()`. It renders inside the `(site)` layout, which
// already has the navigation (unlike `app/not-found.tsx`, used for unmatched URLs).
export default function NotFound() {
  return (
    <StatusPage code={404} title="Page not found">
      <p>There&apos;s nothing here.</p>
      <Link href="/">Return home</Link>
    </StatusPage>
  );
}
