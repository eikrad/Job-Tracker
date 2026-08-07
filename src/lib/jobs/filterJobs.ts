import type { Job } from "../types";

export function filterJobsBySearch(jobs: Job[], query: string): Job[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return jobs;
  return jobs.filter((job) => {
    const company = job.company.toLowerCase();
    const title = (job.title ?? "").toLowerCase();
    return company.includes(needle) || title.includes(needle);
  });
}
