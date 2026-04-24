import { fetchJobSearchResultPageText } from "../../lib/tauriApi";
import type { NewJob } from "../../lib/types";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { en } from "../../i18n/en";
import { createBaseCaptureDraft } from "./urlCapture";

export type CaptureWarningReason = "fetch_failed" | "extract_failed";

export type BuildCaptureDraftArgs = {
  url: string;
  defaultStatus: string;
  onExtract: (rawText: string) => Promise<ExtractJobInfoResult>;
};

export type BuildCaptureDraftResult = {
  draft: NewJob;
  reason?: CaptureWarningReason;
};

export function captureWarningMessage(reason: CaptureWarningReason): string {
  switch (reason) {
    case "fetch_failed":
      return en.capture.fetchFallbackHint;
    case "extract_failed":
      return en.capture.extractFallbackHint;
  }
}

function mergeDraft(base: NewJob, partial: Partial<NewJob>): NewJob {
  const next = { ...base };
  for (const [key, value] of Object.entries(partial)) {
    if (value == null) continue;
    if (typeof value === "string" && !value.trim()) continue;
    (next as Record<string, unknown>)[key] = value;
  }
  return next;
}

async function safeFetchListingText(url: string): Promise<string | null> {
  try {
    const fetched = await fetchJobSearchResultPageText(url);
    const trimmed = fetched.trim();
    return trimmed || null;
  } catch {
    return null;
  }
}

export async function buildCaptureDraft({
  url,
  defaultStatus,
  onExtract,
}: BuildCaptureDraftArgs): Promise<BuildCaptureDraftResult> {
  const baseDraft = createBaseCaptureDraft(url, defaultStatus);
  const rawText = await safeFetchListingText(url);
  if (!rawText) return { draft: baseDraft, reason: "fetch_failed" };

  const draftWithText: NewJob = { ...baseDraft, raw_text: rawText };
  const extracted = await onExtract(rawText);
  if (!extracted.ok) return { draft: draftWithText, reason: "extract_failed" };

  return { draft: mergeDraft(draftWithText, extracted.partial) };
}
