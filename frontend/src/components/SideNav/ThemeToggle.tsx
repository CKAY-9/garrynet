"use client";

import { useLayoutEffect, useSyncExternalStore, type ReactNode } from "react";
import {
  applyTheme,
  readThemePreference,
  setThemePreference,
  subscribeToThemePreference,
  type ThemePreference,
} from "@/lib/theme";
import { MonitorIcon, MoonIcon, SunIcon } from "./icons";
import styles from "./SideNav.module.scss";

const options: { value: ThemePreference; label: string; icon: ReactNode }[] = [
  { value: "system", label: "System theme", icon: <MonitorIcon /> },
  { value: "light", label: "Light theme", icon: <SunIcon /> },
  { value: "dark", label: "Dark theme", icon: <MoonIcon /> },
];

/** Picks light, dark, or following the system theme. */
export function ThemeToggle() {
  // The server can't see the stored choice, so it renders "system" and the client corrects it
  // after hydration. Only the pressed button changes: the page's colours are already right,
  // set by the inline script in the root layout.
  const preference = useSyncExternalStore(subscribeToThemePreference, readThemePreference, () => "system" as const);

  // Keeps `<html>` in step when another tab changes the theme. It also restores the attribute
  // after the development-only Strict Mode remount, which resets `<html>` to its JSX attributes.
  // Reads storage rather than `preference`, which is "system" during hydration.
  useLayoutEffect(() => applyTheme(readThemePreference()), [preference]);

  return (
    <div className={styles.theme} role="group" aria-label="Theme">
      {options.map(({ value, label, icon }) => (
        <button
          key={value}
          className={styles.themeOption}
          type="button"
          aria-label={label}
          aria-pressed={preference === value}
          title={label}
          onClick={() => setThemePreference(value)}
        >
          {icon}
        </button>
      ))}
    </div>
  );
}
