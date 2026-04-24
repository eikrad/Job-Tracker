import { fetchJobSearchResultPageText } from "../../lib/tauriApi";
import type { NewJob } from "../../lib/types";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { createBaseCaptureDraft } from "./urlCapture";

type BuildCaptureDraftArgs = {
  url: string;
  defaultStatus: string;
  onExtract: (rawText: string) => Promise<ExtractJobInfoResult>;
};

function mergeDraft(base: NewJob, partial: Partial<NewJob>): NewJob {
  const next = { ...base };
  for (const [key, value] of Object.entries(partial)) {
    if (value == null) continue;
    if (typeof value === "string" && !value.trim()) continue;
    (next as Record<string, unknown>)[key] = value;
  }
  return next;
}

export async function buildCaptureDraft({
  url,
  defaultStatus,
  onExtract,
}: BuildCaptureDraftArgs): Promise<{ draft: NewJob; warning: string }> {
  let warning = "";
  let rawText: string | undefined;
  try {
    const fetched = await fetchJobSearchResultPageText(url);
    if (fetched.trim()) rawText = fetched.trim();
  } catch {
    warning = "Could not fetch the page text. You can still complete the fields manually.";
  }

  let draft = createBaseCaptureDraft(url, defaultStatus);
  if (!rawText) return { draft, warning };

  draft = { ...draft, raw_text: rawText };
  const extracted = await onExtract(rawText);
  if (!extracted.ok) {
    if (!warning) warning = extracted.error || "Could not auto-extract fields.";
    return { draft, warning };
  }
  return { draft: mergeDraft(draft, extracted.partial), warning };
}
