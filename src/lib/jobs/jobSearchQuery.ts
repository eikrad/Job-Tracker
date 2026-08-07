const STORAGE_KEY = "jobtracker.jobSearchQuery";

function storageAvailable(): boolean {
  try {
    return typeof localStorage !== "undefined" && typeof localStorage.setItem === "function";
  } catch {
    return false;
  }
}

export function normalizeJobSearchQuery(raw: unknown): string {
  if (typeof raw !== "string") return "";
  return raw.trim();
}

export function loadJobSearchQuery(): string {
  if (!storageAvailable()) return "";
  try {
    return normalizeJobSearchQuery(localStorage.getItem(STORAGE_KEY));
  } catch {
    return "";
  }
}

export function saveJobSearchQuery(query: string): void {
  if (!storageAvailable()) return;
  try {
    localStorage.setItem(STORAGE_KEY, normalizeJobSearchQuery(query));
  } catch {
    /* ignore quota / private mode */
  }
}
