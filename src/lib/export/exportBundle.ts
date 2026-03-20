import type { Job } from "../types";

export function exportJobsAsJson(jobs: Job[]) {
  const blob = new Blob([JSON.stringify(jobs, null, 2)], { type: "application/json" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = "job-tracker-export.json";
  link.click();
  URL.revokeObjectURL(link.href);
}
