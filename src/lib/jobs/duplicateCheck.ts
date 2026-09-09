import type { Job, NewJob } from "../types";

/**
 * Returns an existing job if payload likely duplicates URL or company+title.
 *
 * Follow-up (not in mail-scan PRs): align this with `mailFingerprint` /
 * `tests/fixtures/fingerprints.json` normalization. Changing it here would alter
 * existing job dedup for every user.
 */
export function findDuplicateJob(
  jobs: Job[],
  payload: NewJob,
  excludeJobId?: number,
): Job | undefined {
  return jobs.find((j) => {
    if (excludeJobId != null && j.id === excludeJobId) return false;
    return (
      (!!payload.url && !!j.url && payload.url === j.url) ||
      (j.company.toLowerCase() === payload.company.toLowerCase() &&
        (j.title ?? "").toLowerCase() === (payload.title ?? "").toLowerCase())
    );
  });
}
