import { invoke } from "@tauri-apps/api/core";
import type { Job, NewJob } from "./types";

export async function initDb() {
  return invoke("init_db");
}

export async function listJobs(): Promise<Job[]> {
  return invoke<Job[]>("list_jobs");
}

export async function createJob(payload: NewJob): Promise<number> {
  return invoke<number>("create_job", { payload });
}

export async function updateJobStatus(jobId: number, newStatus: string): Promise<void> {
  return invoke("update_job_status", { jobId, newStatus });
}

export async function listStatusHistory(jobId: number): Promise<
  Array<{ from_status: string | null; to_status: string; changed_at: string }>
> {
  return invoke("list_status_history", { jobId });
}

export async function saveApplicationPdf(
  jobId: number,
  originalName: string,
  bytes: number[],
): Promise<string> {
  return invoke("save_application_pdf", { jobId, originalName, bytes });
}
