import { DEFAULT_STATUSES } from "./types";

/** Active status columns, or defaults when the user list is empty. */
export function effectiveStatuses(statuses: string[]): string[] {
  return statuses.length > 0 ? statuses : [...DEFAULT_STATUSES];
}

/** One-time migration: inserts "Plan to Apply" between "Interesting" and "Application Sent". */
export function migrateStatusesV2(statuses: string[]): string[] {
  if (
    statuses.includes("Interesting") &&
    statuses.includes("Application Sent") &&
    !statuses.includes("Plan to Apply")
  ) {
    const next = [...statuses];
    next.splice(next.indexOf("Application Sent"), 0, "Plan to Apply");
    return next;
  }
  return statuses;
}

/** Returns true if `target` is the immediate next status after `currentStatus` in `lanes`. */
export function isNextStatus(lanes: string[], currentStatus: string, target: string): boolean {
  const idx = lanes.indexOf(currentStatus);
  return idx >= 0 && lanes[idx + 1] === target;
}
