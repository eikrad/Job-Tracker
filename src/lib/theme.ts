/**
 * Theme controller for the KDE/Manjaro "Breath" light & dark palettes.
 *
 * Three preferences are supported:
 *  - "system": follow the OS / Plasma color scheme (auto light/dark)
 *  - "light":  force Breath Light
 *  - "dark":   force Breath Dark
 *
 * The resolved theme ("light" | "dark") is written to
 * `document.documentElement[data-theme]`, which the CSS in index.css reads.
 */

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";

const STORAGE_KEY = "jobtracker.theme";

function media(): MediaQueryList | null {
  if (typeof window === "undefined" || !window.matchMedia) return null;
  return window.matchMedia("(prefers-color-scheme: dark)");
}

export function getStoredPreference(): ThemePreference {
  if (typeof localStorage === "undefined") return "system";
  const value = localStorage.getItem(STORAGE_KEY);
  if (value === "light" || value === "dark" || value === "system") return value;
  return "system";
}

export function storePreference(pref: ThemePreference): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, pref);
}

export function systemTheme(): ResolvedTheme {
  return media()?.matches ? "dark" : "light";
}

export function resolveTheme(pref: ThemePreference): ResolvedTheme {
  return pref === "system" ? systemTheme() : pref;
}

/** Apply a preference to the document root and return the resolved theme. */
export function applyTheme(pref: ThemePreference): ResolvedTheme {
  const resolved = resolveTheme(pref);
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = resolved;
  }
  return resolved;
}

/** Read the stored preference and apply it. Call once, before first paint. */
export function initTheme(): ResolvedTheme {
  return applyTheme(getStoredPreference());
}

/**
 * Subscribe to OS color-scheme changes. The callback fires with the new
 * resolved system theme whenever Plasma toggles light/dark. Returns an
 * unsubscribe function.
 */
export function subscribeToSystemTheme(cb: (theme: ResolvedTheme) => void): () => void {
  const mql = media();
  if (!mql) return () => {};
  const handler = () => cb(mql.matches ? "dark" : "light");
  mql.addEventListener("change", handler);
  return () => mql.removeEventListener("change", handler);
}
