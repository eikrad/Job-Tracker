import { DEFAULT_STATUSES } from "./types";

/** Active status columns, or defaults when the user list is empty. */
export function effectiveStatuses(statuses: string[]): string[] {
  return statuses.length > 0 ? statuses : [...DEFAULT_STATUSES];
}
