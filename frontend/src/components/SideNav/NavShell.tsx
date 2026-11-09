"use client";

import { useState, type MouseEvent, type ReactNode } from "react";
import { CloseIcon, MenuIcon } from "./icons";
import styles from "./SideNav.module.scss";

/**
 * Frame around the navigation content. On wide screens it is a full-height sidebar; on
 * narrow screens it collapses behind a hamburger button and opens as an overlay.
 */
export function NavShell({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);

  // Close the overlay when any link inside it is followed. (Reading the pathname instead
  // would keep the nav out of the prerendered shell on dynamic routes.)
  const closeOnLink = (e: MouseEvent<HTMLElement>) => {
    if ((e.target as HTMLElement).closest("a")) setOpen(false);
  };

  return (
    <>
      <button
        className={styles.hamburger}
        type="button"
        aria-label="Open navigation"
        aria-expanded={open}
        onClick={() => setOpen(true)}
      >
        <MenuIcon />
      </button>
      <nav
        className={`${styles.nav} ${open ? styles.open : ""}`}
        aria-label="Main"
        onClick={closeOnLink}
        // The search form navigates too.
        onSubmit={() => setOpen(false)}
      >
        <button
          className={styles.close}
          type="button"
          aria-label="Close navigation"
          onClick={() => setOpen(false)}
        >
          <CloseIcon />
        </button>
        {children}
      </nav>
    </>
  );
}
