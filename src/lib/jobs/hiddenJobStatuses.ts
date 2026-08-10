import { readStoredJson, writeStoredJson } from "../storage/localStoragePref";

const STORAGE_KEY = "jobtracker.hiddenJobStatuses";

export function normalizeHiddenJobStatuses(raw: unknown, pipeline: string[]): string[] {
  if (!Array.isArray(raw)) return [];
  const allowed = new Set(pipeline);
  const picked = raw.filter((name): name is string => typeof name === "string" && allowed.has(name));
  return pipeline.filter((name) => picked.includes(name));
}

export function loadHiddenJobStatuses(pipeline: string[]): string[] {
  return normalizeHiddenJobStatuses(readStoredJson(STORAGE_KEY), pipeline);
}

export function saveHiddenJobStatuses(statuses: string[]): void {
  writeStoredJson(STORAGE_KEY, statuses);
}

export function toggleHiddenJobStatus(
  current: string[],
  status: string,
  pipeline: string[],
): string[] {
  const next = current.includes(status)
    ? current.filter((name) => name !== status)
    : [...current, status];
  return normalizeHiddenJobStatuses(next, pipeline);
}
