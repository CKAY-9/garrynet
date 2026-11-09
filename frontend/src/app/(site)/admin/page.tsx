import { Suspense } from "react";
import { StatCard, StatCardSkeleton } from "@/components/StatCard";
import { getAdminStats } from "@/lib/admin";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import styles from "./admin.module.scss";

export default function AdminDashboard() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense fallback={<DashboardSkeleton />}>
      <Dashboard />
    </Suspense>
  );
}

async function Dashboard() {
  await requirePermission(Permission.AdminAccess);
  const stats = await getAdminStats();

  return (
    <>
      <section className={styles.section} aria-labelledby="users-heading">
        <h2 id="users-heading" className={styles.sectionTitle}>
          Users
        </h2>
        <dl className={styles.stats}>
          <StatCard label="Registered users" value={stats.users.registered} />
          <StatCard label="Active users" value={stats.users.active} detail="Online right now" />
        </dl>
      </section>

      <section className={styles.section} aria-labelledby="forum-heading">
        <h2 id="forum-heading" className={styles.sectionTitle}>
          Forum
        </h2>
        <dl className={styles.stats}>
          <StatCard label="Posts" value={stats.forum.posts_week} detail="Past 7 days" />
        </dl>
      </section>

      <section className={styles.section} aria-labelledby="store-heading">
        <h2 id="store-heading" className={styles.sectionTitle}>
          Store
        </h2>
        <dl className={styles.stats}>
          {/* TODO: real data once the store exists (purchases and income over the past week). */}
          <StatCard label="Purchases" detail="Past 7 days" todo />
          <StatCard label="Income" detail="Past 7 days" todo />
        </dl>
      </section>
    </>
  );
}

function DashboardSkeleton() {
  return (
    <dl className={styles.stats} aria-label="Loading statistics">
      <StatCardSkeleton label="Registered users" />
      <StatCardSkeleton label="Active users" />
    </dl>
  );
}
