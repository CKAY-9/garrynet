import { Suspense } from "react";
import type { Metadata } from "next";
import { AdminNav, AdminNavSkeleton } from "@/components/AdminNav";
import styles from "./admin.module.scss";

export const metadata: Metadata = {
  title: "Admin · GarryNet",
};

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
