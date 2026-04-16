import type { NewJob } from "../../lib/types";
import type { JobSearchResult } from "../../lib/tauriApi";
import { en } from "../../i18n/en";

export async function buildSavedJobPayload(
  result: JobSearchResult,
  fetchPageText: (url: string) => Promise<string>,
): Promise<NewJob> {
  let rawText = result.description.trim();

  if (result.url.trim()) {
    try {
      const fetchedText = await fetchPageText(result.url);
      if (fetchedText.trim()) {
        rawText = fetchedText.trim();
      }
    } catch (error) {
      console.warn("Could not fetch full job ad text:", error);
    }
  }

  const tags = new Set<string>();
  if (result.platform.trim()) {
    tags.add(result.platform.trim().toLowerCase());
  }
  if (result.location.trim()) {
    result.location
      .split(/[,\-/|]/)
      .map((part) => part.trim().toLowerCase())
      .filter(Boolean)
      .forEach((part) => tags.add(part));
  }

  return {
    company: result.company || en.jobSearch.unknownCompany,
    title: result.title || undefined,
    url: result.url || undefined,
    raw_text: rawText || undefined,
    status: "Interesting",
    tags: tags.size > 0 ? Array.from(tags).join(", ") : undefined,
    source: result.platform || undefined,
    workplace_city: result.location || undefined,
  };
}
