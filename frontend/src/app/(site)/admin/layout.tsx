import { Suspense } from "react";
import type { Metadata } from "next";
import { AdminNav, AdminNavSkeleton } from "@/components/AdminNav";
import { FALLBACK_COMMUNITY_NAME, getCommunity } from "@/lib/community";
import styles from "./admin.module.scss";

/** "Admin · <community>" here, "Usergroups · Admin · <community>" on the pages under it. */
export async function generateMetadata(): Promise<Metadata> {
  const name = (await getCommunity())?.name ?? FALLBACK_COMMUNITY_NAME;
  return { title: { default: "Admin", template: `%s · Admin · ${name}` } };
}

/**
 * Frame for every admin page: a heading and the admin tabs. Each page checks its own
 * permissions with `requirePermission`.
 */
export default function AdminLayout({ children }: LayoutProps<"/admin">) {
  return (
    <div className={styles.admin}>
      <header className={styles.header}>
        <h1 className={styles.title}>Admin</h1>
        {/* Depends on the session cookie, so it streams in behind the boundary. */}
        <Suspense fallback={<AdminNavSkeleton />}>
          <AdminNav />
        </Suspense>
      </header>
      {children}
    </div>
  );
}
