import { useCallback, useEffect, useState } from "react";
import {
  applyTheme,
  getStoredPreference,
  storePreference,
  subscribeToSystemTheme,
  systemTheme,
  type ResolvedTheme,
  type ThemePreference,
} from "../lib/theme";

/**
 * React access to the app theme. Exposes the user's preference
 * (system/light/dark), the currently resolved theme, and a setter that
 * persists the choice and updates the document.
 */
export function useTheme() {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    getStoredPreference(),
  );
  // Tracks the OS-resolved theme; updated by the media-query subscription.
  // Only influences the output when preference === "system".
  const [osTheme, setOsTheme] = useState<ResolvedTheme>(() => systemTheme());

  // DOM side effect: re-apply theme class whenever preference or OS theme changes.
  useEffect(() => {
    applyTheme(preference);
  }, [preference, osTheme]);

  // When following the system, react to OS light/dark switches live.
  useEffect(() => {
    if (preference !== "system") return;
    return subscribeToSystemTheme(setOsTheme);
  }, [preference]);

  const setPreference = useCallback((pref: ThemePreference) => {
    storePreference(pref);
    setPreferenceState(pref);
  }, []);

  const resolved: ResolvedTheme = preference === "system" ? osTheme : preference;
  return { preference, resolved, setPreference };
}
