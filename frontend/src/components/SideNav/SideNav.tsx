import { Suspense } from "react";
import { Account, AccountSkeleton } from "@/components/Account";
import { getCurrentUser } from "@/lib/auth";
import { hasPermission, Permission } from "@/lib/permissions";
import { CommunityBrand, CommunityBrandSkeleton } from "./CommunityBrand";
import { NavLink } from "./NavLink";
import { NavShell } from "./NavShell";
import { AdminIcon, ForumsIcon, HomeIcon, StoreIcon, SupportIcon } from "./icons";
import styles from "./SideNav.module.scss";

const links = [
  { href: "/", label: "Home", icon: <HomeIcon /> },
  { href: "/forum", label: "Forums", icon: <ForumsIcon /> },
  { href: "/store", label: "Store", icon: <StoreIcon /> },
  { href: "/support", label: "Support", icon: <SupportIcon /> },
];

/**
 * Site-wide navigation: the community's name and logo, general links, then user-specific links
 * at the bottom. A sidebar on wide screens, a hamburger-triggered overlay on narrow ones.
 */
export function SideNav() {
  return (
    <NavShell>
      <div className={styles.top}>
        {/* Fetched at request time, so it streams in behind the boundary. */}
        <Suspense fallback={<CommunityBrandSkeleton />}>
          <CommunityBrand />
        </Suspense>
        <ul className={styles.links}>
          {links.map(({ href, label, icon }) => (
            <li key={href}>
              <NavLink href={href} icon={icon}>
                {label}
              </NavLink>
            </li>
          ))}
          {/* Reads the session cookie, so it streams in behind the boundary. */}
          <Suspense>
            <AdminLink />
          </Suspense>
        </ul>
      </div>
      <div className={styles.user}>
        {/* Reads the session cookie, so it streams in behind the boundary. */}
        <Suspense fallback={<AccountSkeleton />}>
          <Account />
        </Suspense>
      </div>
    </NavShell>
  );
}

/** Link to the admin dashboard, only for users with Admin Access. */
async function AdminLink() {
  const user = await getCurrentUser();
  if (!user || !hasPermission(user.permissions, Permission.AdminAccess)) return null;
  return (
    <li>
      <NavLink href="/admin" icon={<AdminIcon />}>
        Admin
      </NavLink>
    </li>
  );
}
