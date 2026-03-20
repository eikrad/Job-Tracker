import type { NewJob } from "../../lib/types";

/** LLM used for “Extract” on the job form (client-side API calls). */
export type LlmProvider = "gemini" | "mistral";

/** Default chat model for Mistral (works on free tier; see Mistral pricing/docs). */
export const MISTRAL_EXTRACTION_MODEL = "mistral-small-latest";

export type ExtractJobInfoResult =
  | { ok: true; partial: Partial<NewJob> }
  | { ok: false; error: string };

const PARSE_JSON_ERROR = "Could not parse JSON from the model response.";

function buildExtractionPrompt(rawText: string): string {
  return `Extract structured job information from the text below.
Input can be Danish, German, or English.
Return strict JSON with keys:
company,title,url,deadline,tags,detected_language,notes
Use English field values when possible for normalized output.

Text:
${rawText}`;
}

/** Parse model output; tolerate optional markdown fences. */
export function parsePartialNewJobFromLlmText(text: string): Partial<NewJob> {
  const trimmed = text.trim();
  const unfenced = trimmed.replace(/^```(?:json)?\s*/i, "").replace(/\s*```\s*$/i, "");
  for (const candidate of [unfenced, trimmed]) {
    try {
      return JSON.parse(candidate) as Partial<NewJob>;
    } catch {
      /* try next */
    }
  }
  return {};
}

function extractionFromModelText(modelText: string): ExtractJobInfoResult {
  const partial = parsePartialNewJobFromLlmText(modelText);
  return Object.keys(partial).length === 0
    ? { ok: false, error: PARSE_JSON_ERROR }
    : { ok: true, partial };
}

async function readHttpErrorMessage(res: Response): Promise<string> {
  const prefix = `HTTP ${res.status}`;
  let raw = "";
  try {
    raw = await res.text();
  } catch {
    return prefix;
  }
  const fallback = raw.trim() ? `${prefix}: ${raw.slice(0, 400)}` : prefix;
  if (!raw.trim()) return prefix;
  try {
    const j = JSON.parse(raw) as Record<string, unknown>;
    const errObj = j.error;
    if (errObj && typeof errObj === "object" && "message" in errObj) {
      return `${prefix}: ${String((errObj as { message: unknown }).message)}`;
    }
    if (typeof j.message === "string") return `${prefix}: ${j.message}`;
    if (typeof j.detail === "string") return `${prefix}: ${j.detail}`;
  } catch {
    return fallback;
  }
  return fallback;
}

async function extractJobInfoWithGemini(rawText: string, apiKey: string): Promise<ExtractJobInfoResult> {
  const prompt = buildExtractionPrompt(rawText);
  const res = await fetch(
    "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent",
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-goog-api-key": apiKey,
      },
      body: JSON.stringify({
        contents: [{ parts: [{ text: prompt }] }],
        generationConfig: { responseMimeType: "application/json" },
      }),
    },
  );
  if (!res.ok) return { ok: false, error: await readHttpErrorMessage(res) };
  const json: unknown = await res.json();
  const text =
    json &&
    typeof json === "object" &&
    "candidates" in json &&
    Array.isArray((json as { candidates: unknown[] }).candidates) &&
    (json as { candidates: Array<{ content?: { parts?: Array<{ text?: string }> } }> }).candidates[0]
      ?.content?.parts?.[0]?.text;
  if (typeof text !== "string" || !text.trim()) {
    return { ok: false, error: "Gemini returned no text (check API key, quota, or safety filters)." };
  }
  return extractionFromModelText(text);
}

async function extractJobInfoWithMistral(rawText: string, apiKey: string): Promise<ExtractJobInfoResult> {
  const prompt = buildExtractionPrompt(rawText);
  const res = await fetch("https://api.mistral.ai/v1/chat/completions", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${apiKey.trim()}`,
    },
    body: JSON.stringify({
      model: MISTRAL_EXTRACTION_MODEL,
      messages: [{ role: "user", content: prompt }],
      response_format: { type: "json_object" },
      temperature: 0.2,
    }),
  });
  if (!res.ok) return { ok: false, error: await readHttpErrorMessage(res) };
  const json: unknown = await res.json();
  const content =
    json &&
    typeof json === "object" &&
    "choices" in json &&
    Array.isArray((json as { choices: unknown[] }).choices) &&
    (json as { choices: Array<{ message?: { content?: string | null } }> }).choices[0]?.message?.content;
  if (typeof content !== "string" || !content.trim()) {
    return { ok: false, error: "Mistral returned no message content." };
  }
  return extractionFromModelText(content);
}

/**
 * Calls the selected provider’s HTTP API from the browser (key stays in localStorage only).
 */
export async function extractJobInfo(
  rawText: string,
  provider: LlmProvider,
  apiKey: string,
): Promise<ExtractJobInfoResult> {
  if (!apiKey.trim()) {
    return { ok: false, error: "Add an API key in Settings (Job Tracker)." };
  }
  if (!rawText.trim()) {
    return { ok: false, error: "Paste job ad text before extracting." };
  }
  try {
    switch (provider) {
      case "gemini":
        return await extractJobInfoWithGemini(rawText, apiKey);
      case "mistral":
        return await extractJobInfoWithMistral(rawText, apiKey);
    }
  } catch (e) {
    return { ok: false, error: e instanceof Error ? e.message : String(e) };
  }
}
