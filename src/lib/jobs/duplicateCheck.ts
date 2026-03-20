import type { Job, NewJob } from "../types";

/** Returns an existing job if payload likely duplicates URL or company+title. */
export function findDuplicateJob(jobs: Job[], payload: NewJob): Job | undefined {
  return jobs.find(
    (j) =>
      (!!payload.url && !!j.url && payload.url === j.url) ||
      (j.company.toLowerCase() === payload.company.toLowerCase() &&
        (j.title ?? "").toLowerCase() === (payload.title ?? "").toLowerCase()),
  );
}
