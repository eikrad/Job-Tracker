import type { Job } from "../types";

export type JobFilterOptions = {
  query?: string;
  hiddenStatuses?: string[];
};

export function filterJobsBySearch(jobs: Job[], query: string): Job[] {
  return filterJobs(jobs, { query });
}

export function filterJobsByHiddenStatuses(jobs: Job[], hiddenStatuses: string[]): Job[] {
  return filterJobs(jobs, { hiddenStatuses });
}

/** Apply dashboard/table filters. Empty query and empty hidden list are no-ops. */
export function filterJobs(jobs: Job[], options: JobFilterOptions = {}): Job[] {
  const needle = (options.query ?? "").trim().toLowerCase();
  const hidden =
    options.hiddenStatuses && options.hiddenStatuses.length > 0
      ? new Set(options.hiddenStatuses)
      : null;

  if (!needle && !hidden) return jobs;

  return jobs.filter((job) => {
    if (hidden?.has(job.status)) return false;
    if (!needle) return true;
    const company = job.company.toLowerCase();
    const title = (job.title ?? "").toLowerCase();
    return company.includes(needle) || title.includes(needle);
  });
}
