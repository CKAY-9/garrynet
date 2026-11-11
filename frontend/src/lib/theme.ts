/*
 * Site theme. Users can force light or dark; with no stored choice the site follows the
 * system theme through `prefers-color-scheme`. The choice lives in `localStorage` (not a
 * cookie, which the root layout would have to read, taking every page out of the static
 * shell) and is applied as `data-theme` on `<html>`, which `globals.scss` and the `dark`
 * mixin in `src/styles/_theme.scss` key off.
 */

export type Theme = "light" | "dark";
/** What the user picked: a fixed theme, or whatever their system uses. */
export type ThemePreference = Theme | "system";

const STORAGE_KEY = "garrynet_theme";
/** Fired on `window` when this tab changes the theme (`storage` only fires in other tabs). */
const CHANGE_EVENT = "garrynet-theme-change";

const isTheme = (value: unknown): value is Theme => value === "light" || value === "dark";

/**
 * Runs inline in `<head>`, before the first paint, so a stored theme never flashes the
 * other one. Keep it in sync with `applyTheme()`.
 */
export const themeScript = `try{var t=localStorage.getItem(${JSON.stringify(STORAGE_KEY)});if(t==="light"||t==="dark")document.documentElement.setAttribute("data-theme",t)}catch(e){}`;

/** The stored preference. Storage can be unavailable (privacy modes), which reads as "system". */
export function readThemePreference(): ThemePreference {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    return isTheme(stored) ? stored : "system";
  } catch {
    return "system";
  }
}

/** Sets or clears `data-theme` on `<html>` to match a preference. */
export function applyTheme(preference: ThemePreference) {
  const root = document.documentElement;
  if (preference === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", preference);
}

/** Stores and applies a preference, and tells this tab's listeners. */
export function setThemePreference(preference: ThemePreference) {
  try {
    if (preference === "system") localStorage.removeItem(STORAGE_KEY);
    else localStorage.setItem(STORAGE_KEY, preference);
  } catch {
    // Still apply it for this page view.
  }
  applyTheme(preference);
  window.dispatchEvent(new Event(CHANGE_EVENT));
}

/** Calls `onChange` when the preference changes in this tab or another. Returns an unsubscribe. */
export function subscribeToThemePreference(onChange: () => void) {
  const onStorage = (e: StorageEvent) => {
    if (e.key === STORAGE_KEY || e.key === null) onChange();
  };
  window.addEventListener("storage", onStorage);
  window.addEventListener(CHANGE_EVENT, onChange);
  return () => {
    window.removeEventListener("storage", onStorage);
    window.removeEventListener(CHANGE_EVENT, onChange);
  };
}
