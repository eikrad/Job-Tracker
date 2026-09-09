import { invoke } from "@tauri-apps/api/core";
import type { NewJob } from "../../lib/types";

/** LLM used for “Extract” on the job form (keys live in the OS keyring; calls run in Rust). */
export type LlmProvider = "scaleway_deepseek" | "gemini" | "mistral";

export type ExtractJobInfoResult =
  | { ok: true; partial: Partial<NewJob> }
  | { ok: false; error: string };

const PARSE_JSON_ERROR = "Could not parse JSON from the model response.";
const DESKTOP_REQUIRED =
  "AI extraction requires the desktop app (`npm run tauri:dev`), not browser-only Vite.";

/**
 * Job fields the extractor may fill. Rust (`llm::normalize`) owns prompt wording, key
 * aliasing and value normalization; this list is only the IPC trust boundary, turning
 * an untyped payload into `Partial<NewJob>`.
 *
 * `priority` is deliberately absent — it is manual-only and must never come from an LLM.
 */
const EXTRACTABLE_FIELDS = [
  "company",
  "title",
  "url",
  "deadline",
  "interview_date",
  "start_date",
  "tags",
  "detected_language",
  "notes",
  "contact_name",
  "contact_email",
  "contact_phone",
  "workplace_street",
  "workplace_city",
  "workplace_postal_code",
  "work_mode",
  "salary_range",
  "contract_type",
  "reference_number",
  "source",
] as const satisfies readonly (keyof NewJob)[];

/** Keep known, non-empty string fields; drop everything else. */
export function toJobPartial(raw: Record<string, unknown>): Partial<NewJob> {
  const out: Partial<NewJob> = {};
  for (const field of EXTRACTABLE_FIELDS) {
    const value = raw[field];
    if (typeof value === "string" && value.trim()) {
      out[field] = value.trim();
    }
  }
  return out;
}

type RustExtractResponse = {
  ok: boolean;
  partial?: Record<string, unknown>;
  error?: string;
};

/**
 * Calls Rust `extract_job_info` (key from OS keyring). Browser-only Vite has no Tauri.
 */
export async function extractJobInfo(
  rawText: string,
  provider: LlmProvider,
): Promise<ExtractJobInfoResult> {
  if (!rawText.trim()) {
    return { ok: false, error: "Paste job ad text before extracting." };
  }
  try {
    const res = await invoke<RustExtractResponse>("extract_job_info", {
      rawText,
      provider,
    });
    if (!res.ok) {
      return { ok: false, error: res.error ?? "Extraction failed." };
    }
    const partial = toJobPartial(res.partial ?? {});
    return Object.keys(partial).length === 0
      ? { ok: false, error: PARSE_JSON_ERROR }
      : { ok: true, partial };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (/not allowed|webview|tauri|ipc|invoke/i.test(msg) || msg.includes("undefined")) {
      return { ok: false, error: DESKTOP_REQUIRED };
    }
    return { ok: false, error: msg };
  }
}
