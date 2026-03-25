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
company, title, url, deadline, interview_date, start_date, tags, detected_language, notes,
contact_name, contact_email, contact_phone,
workplace_street, workplace_city, workplace_postal_code,
work_mode (one of: Remote, Hybrid, On-site, or omit if unknown),
salary_range (free text as stated in the ad, or omit if not mentioned),
contract_type (one of: Permanent, Fixed-term, Freelance, Internship, or omit if unknown),
reference_number (job reference/ID if mentioned, else omit),
source (where the job was posted if mentioned, else omit)
Dates as YYYY-MM-DD when known.
Use English field values when possible for normalized output.

Text:
${rawText}`;
}

function parseRawJsonObject(text: string): Record<string, unknown> | null {
  const trimmed = text.trim();
  const unfenced = trimmed.replace(/^```(?:json)?\s*/i, "").replace(/\s*```\s*$/i, "");
  for (const candidate of [unfenced, trimmed]) {
    try {
      const v = JSON.parse(candidate) as unknown;
      if (v && typeof v === "object" && !Array.isArray(v)) return v as Record<string, unknown>;
    } catch {
      /* try next */
    }
  }
  return null;
}

function strField(raw: Record<string, unknown>, keys: string[]): string | undefined {
  for (const k of keys) {
    const v = raw[k];
    if (typeof v === "string" && v.trim()) return v.trim();
  }
  return undefined;
}

/**
 * Map messy LLM JSON (alternate key casings, a few aliases) into Partial<NewJob>.
 */
export function normalizeLlmJobPartial(raw: Record<string, unknown>): Partial<NewJob> {
  const out: Partial<NewJob> = {};
  const company = strField(raw, ["company", "Company", "employer", "Employer", "organization"]);
  if (company) out.company = company;
  const title = strField(raw, ["title", "Title", "role", "Role", "job_title", "position"]);
  if (title) out.title = title;
  const url = strField(raw, ["url", "URL", "link", "job_url", "apply_url"]);
  if (url) out.url = url;
  const deadline = strField(raw, ["deadline", "Deadline", "closing_date", "apply_by"]);
  if (deadline) out.deadline = deadline;
  const interview_date = strField(raw, [
    "interview_date",
    "interviewDate",
    "InterviewDate",
    "interviews",
    "interview",
    "assessment_date",
  ]);
  if (interview_date) out.interview_date = interview_date;
  const start_date = strField(raw, [
    "start_date",
    "startDate",
    "StartDate",
    "position_start",
    "role_start",
    "contract_start",
  ]);
  if (start_date) out.start_date = start_date;
  let tags = strField(raw, ["tags", "Tags", "keywords"]);
  const tagsArr = raw.tags;
  if (!tags && Array.isArray(tagsArr)) {
    const joined = tagsArr.filter((x) => typeof x === "string").join(", ");
    if (joined.trim()) tags = joined.trim();
  }
  if (tags) out.tags = tags;
  const detected_language = strField(raw, [
    "detected_language",
    "DetectedLanguage",
    "language",
    "Language",
  ]);
  if (detected_language) out.detected_language = detected_language;
  const notes = strField(raw, ["notes", "Notes", "summary"]);
  if (notes) out.notes = notes;
  const contact_name = strField(raw, ["contact_name", "contactName", "contact"]);
  if (contact_name) out.contact_name = contact_name;
  const contact_email = strField(raw, ["contact_email", "contactEmail", "email"]);
  if (contact_email) out.contact_email = contact_email;
  const contact_phone = strField(raw, ["contact_phone", "contactPhone", "phone"]);
  if (contact_phone) out.contact_phone = contact_phone;
  const workplace_street = strField(raw, ["workplace_street", "street", "address"]);
  if (workplace_street) out.workplace_street = workplace_street;
  const workplace_city = strField(raw, ["workplace_city", "city"]);
  if (workplace_city) out.workplace_city = workplace_city;
  const workplace_postal_code = strField(raw, ["workplace_postal_code", "postalCode", "postal_code", "zip"]);
  if (workplace_postal_code) out.workplace_postal_code = workplace_postal_code;
  const work_mode = strField(raw, ["work_mode", "workMode", "remote", "location_type"]);
  if (work_mode) out.work_mode = work_mode;
  const salary_range = strField(raw, ["salary_range", "salaryRange", "salary", "compensation"]);
  if (salary_range) out.salary_range = salary_range;
  const contract_type = strField(raw, ["contract_type", "contractType", "employment_type", "contract"]);
  if (contract_type) out.contract_type = contract_type;
  // priority is intentionally excluded — manual only, never set from LLM output
  const reference_number = strField(raw, ["reference_number", "referenceNumber", "ref", "job_ref", "job_id"]);
  if (reference_number) out.reference_number = reference_number;
  const source = strField(raw, ["source", "Source", "job_source", "platform"]);
  if (source) out.source = source;
  return out;
}

/** Parse model output; tolerate optional markdown fences and normalize keys. */
export function parsePartialNewJobFromLlmText(text: string): Partial<NewJob> {
  const raw = parseRawJsonObject(text);
  if (!raw) return {};
  return normalizeLlmJobPartial(raw);
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
