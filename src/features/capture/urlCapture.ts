import type { NewJob } from "../../lib/types";

export function normalizeCaptureUrl(input: string): string | null {
  const trimmed = input.trim();
  if (!trimmed) return null;
  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return null;
    return parsed.toString();
  } catch {
    return null;
  }
}

export function sourceFromUrl(url: string): string | undefined {
  try {
    const hostname = new URL(url).hostname.toLowerCase();
    return hostname.startsWith("www.") ? hostname.slice(4) : hostname;
  } catch {
    return undefined;
  }
}

export function createBaseCaptureDraft(url: string, status: string): NewJob {
  return {
    company: "",
    status,
    url,
    source: sourceFromUrl(url),
  };
}
