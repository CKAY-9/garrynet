import { Suspense } from "react";
import { Account, AccountSkeleton } from "@/components/Account";
import { NavNotifications } from "@/components/Notifications";
import { getCurrentUser } from "@/lib/auth";
import { hasPermission, Permission } from "@/lib/permissions";
import { CommunityBrand, CommunityBrandSkeleton } from "./CommunityBrand";
import { NavLink } from "./NavLink";
import { NavSearch } from "./NavSearch";
import { NavShell } from "./NavShell";
import { AdminIcon, ForumsIcon, HomeIcon, StoreIcon, SupportIcon, UsersIcon } from "./icons";
import styles from "./SideNav.module.scss";

const links = [
  { href: "/", label: "Home", icon: <HomeIcon /> },
  { href: "/forum", label: "Forums", icon: <ForumsIcon /> },
  { href: "/store", label: "Store", icon: <StoreIcon /> },
  // Highlighted on profiles too, which live under `/users`.
  { href: "/users/search", activePrefix: "/users", label: "Users", icon: <UsersIcon /> },
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
        <NavSearch />
        <ul className={styles.links}>
          {links.map(({ href, activePrefix, label, icon }) => (
            <li key={href}>
              <NavLink href={href} activePrefix={activePrefix} icon={icon}>
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
        {/* Both read the session cookie, so they stream in behind their boundaries. */}
        <Suspense>
          <NavNotifications />
        </Suspense>
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
