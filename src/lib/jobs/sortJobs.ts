import type { Job } from "../types";

export type JobSortKey =
  | "company"
  | "title"
  | "status"
  | "priority"
  | "created_at"
  | "deadline"
  | "interview_date"
  | "start_date"
  | "detected_language";

export type SortDirection = "desc" | "asc";

export type JobSortConfig = {
  primary: JobSortKey;
  primaryDirection: SortDirection;
  secondary: JobSortKey | null;
  secondaryDirection: SortDirection;
  statusOrder?: string[];
};

export function compareNullableStrings(a: string | null | undefined, b: string | null | undefined) {
  const left = (a ?? "").trim().toLocaleLowerCase();
  const right = (b ?? "").trim().toLocaleLowerCase();
  if (!left && !right) return 0;
  if (!left) return 1;
  if (!right) return -1;
  return left.localeCompare(right);
}

export function compareNullableDates(a: string | null | undefined, b: string | null | undefined) {
  if (!a && !b) return 0;
  if (!a) return 1;
  if (!b) return -1;
  return a.localeCompare(b);
}

function statusRank(status: string, statusOrder: string[]) {
  const idx = statusOrder.indexOf(status);
  return idx >= 0 ? idx : statusOrder.length;
}

export function compareJobsByKey(
  a: Job,
  b: Job,
  key: JobSortKey,
  direction: SortDirection,
  statusOrder: string[],
): number {
  let result: number;
  switch (key) {
    case "priority": {
      const aPriority = a.priority ?? -1;
      const bPriority = b.priority ?? -1;
      result = aPriority - bPriority;
      break;
    }
    case "status":
      result = statusRank(a.status, statusOrder) - statusRank(b.status, statusOrder);
      break;
    case "created_at":
    case "deadline":
    case "interview_date":
    case "start_date":
      result = compareNullableDates(a[key], b[key]);
      break;
    case "company":
    case "title":
    case "detected_language":
      result = compareNullableStrings(a[key], b[key]);
      break;
    default: {
      const _never: never = key;
      return _never;
    }
  }
  if (result === 0) return 0;
  return direction === "desc" ? -result : result;
}

export function sortJobs(jobs: Job[], config: JobSortConfig): Job[] {
  const statusOrder = config.statusOrder ?? [];
  const copy = [...jobs];
  copy.sort((a, b) => {
    const primary = compareJobsByKey(a, b, config.primary, config.primaryDirection, statusOrder);
    if (primary !== 0) return primary;

    if (config.secondary && config.secondary !== config.primary) {
      const secondary = compareJobsByKey(
        a,
        b,
        config.secondary,
        config.secondaryDirection,
        statusOrder,
      );
      if (secondary !== 0) return secondary;
    }

    return b.id - a.id;
  });
  return copy;
}

export function formatJobAddedDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso.slice(0, 10);
  return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
}
