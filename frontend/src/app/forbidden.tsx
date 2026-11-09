import Link from "next/link";
import { StatusPage } from "@/components/StatusPage";

export default function Forbidden() {
  return (
    <StatusPage code={403} title="Access denied">
      <p>Your usergroups don&apos;t have permission to view this page.</p>
      <Link href="/">Return home</Link>
    </StatusPage>
  );
}
