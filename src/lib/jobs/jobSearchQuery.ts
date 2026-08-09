import { readStoredString, writeStoredString } from "../storage/localStoragePref";

const STORAGE_KEY = "jobtracker.jobSearchQuery";

export function normalizeJobSearchQuery(raw: unknown): string {
  if (typeof raw !== "string") return "";
  return raw.trim();
}

export function loadJobSearchQuery(): string {
  return normalizeJobSearchQuery(readStoredString(STORAGE_KEY) ?? "");
}

export function saveJobSearchQuery(query: string): void {
  writeStoredString(STORAGE_KEY, normalizeJobSearchQuery(query));
}
