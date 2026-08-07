const STORAGE_KEY = "jobtracker.hiddenJobStatuses";

function storageAvailable(): boolean {
  try {
    return typeof localStorage !== "undefined" && typeof localStorage.setItem === "function";
  } catch {
    return false;
  }
}

export function normalizeHiddenJobStatuses(raw: unknown, pipeline: string[]): string[] {
  if (!Array.isArray(raw)) return [];
  const allowed = new Set(pipeline);
  const picked = raw.filter((name): name is string => typeof name === "string" && allowed.has(name));
  return pipeline.filter((name) => picked.includes(name));
}

export function loadHiddenJobStatuses(pipeline: string[]): string[] {
  if (!storageAvailable()) return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return normalizeHiddenJobStatuses(JSON.parse(raw), pipeline);
  } catch {
    return [];
  }
}

export function saveHiddenJobStatuses(statuses: string[]): void {
  if (!storageAvailable()) return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(statuses));
  } catch {
    /* ignore quota / private mode */
  }
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
