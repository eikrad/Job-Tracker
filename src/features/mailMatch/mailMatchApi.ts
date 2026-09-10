/** Typed wrappers around the mail-scan Tauri commands. */

import { invoke } from "@tauri-apps/api/core";
import type { NewJob } from "../../lib/types";
import type { MailMatchRow } from "./mailMatchInbox";

export type DismissedRow = {
  fingerprintId: string;
  reason: string | null;
  dismissedAt: string;
  dismissedRun: string | null;
  title: string | null;
  company: string | null;
  listingUrl: string | null;
};

export type RunRow = {
  runId: string;
  status: string;
  startedAt: string;
  finishedAt: string | null;
  statsJson: string;
  errorCode: string | null;
  errorSummary: string | null;
  modelId: string | null;
};

export type SightingRow = {
  pass: number;
  score: number | null;
  reason: string | null;
  outcome: string;
  scoredAt: string;
  modelId: string;
};

export type FieldSuggestion = {
  field: string;
  suggested: string;
  current: string | null;
  applicable: boolean;
};

export type UpdatePreview = {
  inboxId: number;
  jobId: number;
  /** True when the job was edited after the scan computed this suggestion. */
  jobChangedSinceScan: boolean;
  fields: FieldSuggestion[];
};

export type AcceptOutcome = {
  jobId: number;
  fieldsWritten: string[];
  /** False when this call lost the race to a double-click. */
  created: boolean;
};

export type ProfileStatus = {
  configured: boolean;
  sizeBytes?: number;
  modifiedAt?: string;
  hashPrefix?: string;
};

export type CallEstimate = {
  listings: number;
  pass1Calls: number;
  pass2Calls: number;
  enrichCalls: number;
  totalCalls: number;
  overBudget: boolean;
};

export type ScanEstimate = {
  estimate: CallEstimate;
  backlogUnderCutoff: number;
  modelId: string;
  profileShort: ProfileStatus;
  profileFull: ProfileStatus;
};

export type MailSource = { id: string; kind: string; path: string };

export async function mailMatchList(status = "pending"): Promise<MailMatchRow[]> {
  return invoke("mail_match_list", { status });
}

export async function mailMatchListDismissed(): Promise<DismissedRow[]> {
  return invoke("mail_match_list_dismissed");
}

export async function mailScanListRuns(limit = 20): Promise<RunRow[]> {
  return invoke("mail_scan_list_runs", { limit });
}

export async function mailMatchSightings(fingerprintId: string): Promise<SightingRow[]> {
  return invoke("mail_match_sightings", { fingerprintId });
}

export async function mailMatchDismiss(inboxId: number, reason?: string): Promise<void> {
  await invoke("mail_match_dismiss", { inboxId, reason: reason ?? null });
}

export async function mailMatchRestore(fingerprintId: string): Promise<void> {
  await invoke("mail_match_restore", { fingerprintId });
}

export async function mailMatchPreviewUpdate(inboxId: number): Promise<UpdatePreview> {
  return invoke("mail_match_preview_update", { inboxId });
}

export async function mailMatchAcceptUpdate(inboxId: number): Promise<AcceptOutcome> {
  return invoke("mail_match_accept_update", { inboxId });
}

export async function mailMatchAcceptNew(
  inboxId: number,
  payload: NewJob,
): Promise<AcceptOutcome> {
  return invoke("mail_match_accept_new", { inboxId, payload });
}

export async function mailScanEstimate(params: {
  provider?: string;
  expectedListings?: number;
  maxCalls?: number;
}): Promise<ScanEstimate> {
  return invoke("mail_scan_estimate", {
    provider: params.provider ?? null,
    expectedListings: params.expectedListings ?? null,
    maxCalls: params.maxCalls ?? null,
  });
}

export async function mailScanStart(request: {
  sources: MailSource[];
  provider?: string;
  cutoff?: number;
  sinceDays?: number;
  maxCalls?: number;
  forceRescore?: boolean;
}): Promise<string> {
  return invoke("mail_scan_start", { request });
}

export async function mailScanCancel(): Promise<void> {
  await invoke("mail_scan_cancel");
}

export async function mailScanGetEnabled(): Promise<boolean> {
  return invoke("mail_scan_get_enabled");
}

export async function mailScanProfileStatus(kind: "short" | "full"): Promise<ProfileStatus> {
  return invoke("mail_scan_profile_status", { kind });
}

export async function mailScanProfileSetFromPath(
  kind: "short" | "full",
  path: string,
): Promise<ProfileStatus> {
  return invoke("mail_scan_profile_set_from_path", { kind, path });
}

export async function mailScanProfileClear(kind: "short" | "full"): Promise<ProfileStatus> {
  return invoke("mail_scan_profile_clear", { kind });
}
