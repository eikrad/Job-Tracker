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

export async function deleteJob(jobId: number): Promise<void> {
  return invoke("delete_job", { jobId });
}

export async function updateJob(jobId: number, payload: NewJob): Promise<void> {
  return invoke("update_job", { jobId, payload });
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

export async function importJobs(jobs: NewJob[]): Promise<number> {
  return invoke<number>("import_jobs", { jobs });
}

export type GoogleCalendarDateKind = "apply" | "interview" | "start";

export async function googleOauthGetClientId(): Promise<string> {
  return invoke<string>("google_oauth_get_client_id");
}

export async function googleOauthSetClientId(clientId: string): Promise<void> {
  return invoke("google_oauth_set_client_id", { clientId });
}

export async function googleOauthStatus(): Promise<{ connected: boolean }> {
  return invoke<{ connected: boolean }>("google_oauth_status");
}

export async function googleOauthConnect(): Promise<void> {
  return invoke("google_oauth_connect");
}

export async function googleOauthDisconnect(): Promise<void> {
  return invoke("google_oauth_disconnect");
}

export async function googleCalendarCreateEvent(params: {
  jobId: number;
  dateKind: GoogleCalendarDateKind;
  /** Legacy: if set, used instead of stored OAuth refresh token. */
  accessToken?: string | null;
}): Promise<string> {
  return invoke<string>("google_calendar_create_event", {
    args: {
      jobId: params.jobId,
      dateKind: params.dateKind,
      accessToken: params.accessToken?.trim() ? params.accessToken.trim() : null,
    },
  });
}
