import type { NewJob } from "../../lib/types";

export async function extractJobInfoWithGemini(
  rawText: string,
  apiKey: string,
): Promise<Partial<NewJob>> {
  if (!apiKey || !rawText.trim()) return {};
  const prompt = `Extract structured job information from the text below.
Input can be Danish, German, or English.
Return strict JSON with keys:
company,title,url,deadline,tags,detected_language,notes
Use English field values when possible for normalized output.

Text:
${rawText}`;

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
  if (!res.ok) return {};
  const json = await res.json();
  const text = json?.candidates?.[0]?.content?.parts?.[0]?.text;
  if (!text) return {};
  try {
    return JSON.parse(text);
  } catch {
    return {};
  }
}
