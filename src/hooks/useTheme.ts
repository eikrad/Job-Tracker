import { useCallback, useEffect, useState } from "react";
import {
  applyTheme,
  getStoredPreference,
  resolveTheme,
  storePreference,
  subscribeToSystemTheme,
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
  const [resolved, setResolved] = useState<ResolvedTheme>(() =>
    resolveTheme(getStoredPreference()),
  );

  // Re-apply whenever the preference changes.
  useEffect(() => {
    setResolved(applyTheme(preference));
  }, [preference]);

  // When following the system, react to OS light/dark switches live.
  useEffect(() => {
    if (preference !== "system") return;
    return subscribeToSystemTheme((next) => {
      setResolved(next);
      applyTheme("system");
    });
  }, [preference]);

  const setPreference = useCallback((pref: ThemePreference) => {
    storePreference(pref);
    setPreferenceState(pref);
  }, []);

  return { preference, resolved, setPreference };
}
