import type { Job } from "../types";

export function exportJobsAsJson(jobs: Job[]) {
  const blob = new Blob([JSON.stringify(jobs, null, 2)], { type: "application/json" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = "job-tracker-export.json";
  link.click();
  URL.revokeObjectURL(link.href);
}

export function exportJobsAsCsv(jobs: Job[]) {
  const headers = ["id", "company", "title", "status", "deadline", "url", "tags", "detected_language"];
  const lines = jobs.map((j) =>
    [j.id, j.company, j.title ?? "", j.status, j.deadline ?? "", j.url ?? "", j.tags ?? "", j.detected_language ?? ""]
      .map((v) => `"${String(v).replaceAll("\"", "\"\"")}"`)
      .join(","),
  );
  const csv = [headers.join(","), ...lines].join("\n");
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = "job-tracker-export.csv";
  link.click();
  URL.revokeObjectURL(link.href);
}
