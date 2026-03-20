import type { Job, NewJob } from "../types";

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

export const NEW_JOB_KEYS = [
  "company",
  "title",
  "url",
  "raw_text",
  "status",
  "deadline",
  "tags",
  "detected_language",
  "notes",
] as const;

function rowToNewJob(row: Record<string, unknown>): NewJob {
  const str = (v: unknown) => (v == null || v === "" ? undefined : String(v));
  const company = String(row.company ?? "").trim();
  return {
    company: company || "Unknown",
    title: str(row.title),
    url: str(row.url),
    raw_text: str(row.raw_text),
    status: str(row.status) ?? "Interesting",
    deadline: str(row.deadline),
    tags: str(row.tags),
    detected_language: str(row.detected_language),
    notes: str(row.notes),
  };
}

/** Parse exported JSON (array of jobs); ignores id/pdf_path/timestamps for import. */
export function parseJobsImportJson(text: string): NewJob[] {
  const data = JSON.parse(text) as unknown;
  if (!Array.isArray(data)) throw new Error("Expected a JSON array of jobs");
  return data.map((item) => rowToNewJob(item as Record<string, unknown>));
}

/** Minimal CSV import matching export header order. */
export function parseJobsImportCsv(text: string): NewJob[] {
  const lines = text.trim().split(/\r?\n/).filter(Boolean);
  if (lines.length < 2) return [];
  const header = lines[0].split(",").map((h) => h.replace(/^"|"$/g, "").trim());
  const out: NewJob[] = [];
  for (let i = 1; i < lines.length; i++) {
    const cells = lines[i].match(/("([^"]|"")*"|[^,]+)/g) ?? [];
    const vals = cells.map((c) => c.replace(/^"|"$/g, "").replace(/""/g, '"').trim());
    const row: Record<string, string> = {};
    header.forEach((h, j) => {
      row[h] = vals[j] ?? "";
    });
    out.push(
      rowToNewJob({
        company: row.company,
        title: row.title,
        status: row.status,
        deadline: row.deadline,
        url: row.url,
        tags: row.tags,
        detected_language: row.detected_language,
      }),
    );
  }
  return out.filter((j) => j.company && j.company !== "Unknown");
}

export function validateExportJobShape(row: Record<string, unknown>): string[] {
  const missing: string[] = [];
  if (row.company == null || String(row.company).trim() === "") missing.push("company");
  if (row.status == null || String(row.status).trim() === "") missing.push("status");
  return missing;
}
