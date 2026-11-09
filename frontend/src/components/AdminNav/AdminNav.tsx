import { getCurrentUser } from "@/lib/auth";
import { adminPages } from "@/lib/adminPages";
import { hasPermission, Permission } from "@/lib/permissions";
import { AdminNavLink } from "./AdminNavLink";
import styles from "./AdminNav.module.scss";

/**
 * Tabs for the admin sections the current user may open. Renders nothing without Admin
 * Access (the page itself shows the 401/403). Reads the session cookie, so render it inside
 * a `<Suspense>` boundary.
 */
export async function AdminNav() {
  const user = await getCurrentUser();
  if (!user) return null;
  if (!hasPermission(user.permissions, Permission.AdminAccess)) return null;
  const allowed = (permission: number, any = false) =>
    any ? (user.permissions & permission) !== 0 : hasPermission(user.permissions, permission);

  return (
    <nav className={styles.nav} aria-label="Admin">
      <ul className={styles.tabs}>
        {adminPages
          .filter(({ permission, any }) => allowed(permission, any))
          .map(({ href, label }) => (
            <li key={href}>
              <AdminNavLink href={href}>{label}</AdminNavLink>
            </li>
          ))}
      </ul>
    </nav>
  );
}

export function AdminNavSkeleton() {
  return <div className={styles.skeleton} aria-hidden />;
}
