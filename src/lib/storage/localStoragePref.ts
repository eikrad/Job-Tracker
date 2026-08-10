/**
 * Shared localStorage helpers for UI preferences.
 * Domain modules own keys + normalize; this module owns availability and I/O.
 */

export function storageAvailable(): boolean {
  try {
    return typeof localStorage !== "undefined" && typeof localStorage.setItem === "function";
  } catch {
    return false;
  }
}

export function readStoredString(key: string): string | null {
  if (!storageAvailable()) return null;
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writeStoredString(key: string, value: string): void {
  if (!storageAvailable()) return;
  try {
    localStorage.setItem(key, value);
  } catch {
    /* ignore quota / private mode */
  }
}

export function readStoredJson(key: string): unknown | null {
  const raw = readStoredString(key);
  if (raw == null) return null;
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return null;
  }
}

export function writeStoredJson(key: string, value: unknown): void {
  writeStoredString(key, JSON.stringify(value));
}
