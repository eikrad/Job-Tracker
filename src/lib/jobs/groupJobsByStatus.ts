import type { Job } from "../types";

/** Group jobs by status for O(jobs) Kanban rendering instead of filtering per column. */
export function groupJobsByStatus(jobs: Job[]): Map<string, Job[]> {
  const map = new Map<string, Job[]>();
  for (const job of jobs) {
    const list = map.get(job.status);
    if (list) list.push(job);
    else map.set(job.status, [job]);
  }
  return map;
}
