"use client";

import { Suspense, type ReactNode } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import styles from "./SideNav.module.scss";

type Props = {
  href: string;
  icon: ReactNode;
  children: ReactNode;
  /** Highlight the link on any route under this path instead of under `href`. */
  activePrefix?: string;
  /** Shown after the label, e.g. an unread count. */
  badge?: ReactNode;
  /** Overrides the accessible name, e.g. to include the unread count. */
  "aria-label"?: string;
};

/**
 * A side navigation link that highlights itself when its route is active. The pathname
 * isn't known while prerendering dynamic routes (e.g. `/users/[user_id]`), so the link
 * prerenders unhighlighted and picks up its active state at request time.
 */
export function NavLink(props: Props) {
  return (
    <Suspense fallback={<NavLinkView {...props} active={false} />}>
      <ActiveNavLink {...props} />
    </Suspense>
  );
}

function ActiveNavLink(props: Props) {
  const pathname = usePathname();
  const prefix = props.activePrefix ?? props.href;
  const active = prefix === "/" ? pathname === "/" : pathname === prefix || pathname.startsWith(`${prefix}/`);
  return <NavLinkView {...props} active={active} />;
}

function NavLinkView({ href, icon, children, active, badge, "aria-label": ariaLabel }: Props & { active: boolean }) {
  return (
    <Link
      className={`${styles.link} ${active ? styles.active : ""}`}
      href={href}
      aria-current={active ? "page" : undefined}
      aria-label={ariaLabel}
    >
      <span className={styles.icon}>{icon}</span>
      {children}
      {badge}
    </Link>
  );
}
