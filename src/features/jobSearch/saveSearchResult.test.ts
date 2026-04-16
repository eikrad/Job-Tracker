import { describe, expect, it, vi } from "vitest";
import type { JobSearchResult } from "../../lib/tauriApi";
import { buildSavedJobPayload } from "./saveSearchResult";

const baseResult: JobSearchResult = {
  title: "Senior Developer",
  company: "Acme",
  location: "Copenhagen",
  url: "https://jobs.example.com/123",
  description: "Short search snippet",
  published_date: "2026-04-16",
  platform: "jobindex",
  freshness_score: 0.9,
  keyword_score: 0.8,
  total_score: 0.86,
};

describe("buildSavedJobPayload", () => {
  it("keeps the link and prefers fetched job page text", async () => {
    const fetchPageText = vi.fn().mockResolvedValue("Full job advertisement text");

    const payload = await buildSavedJobPayload(baseResult, fetchPageText);

    expect(fetchPageText).toHaveBeenCalledWith(baseResult.url);
    expect(payload.url).toBe(baseResult.url);
    expect(payload.raw_text).toBe("Full job advertisement text");
    expect(payload.company).toBe("Acme");
    expect(payload.title).toBe("Senior Developer");
    expect(payload.status).toBe("Interesting");
  });

  it("falls back to the search snippet when page text fetch fails", async () => {
    const fetchPageText = vi.fn().mockRejectedValue(new Error("blocked"));

    const payload = await buildSavedJobPayload(baseResult, fetchPageText);

    expect(payload.url).toBe(baseResult.url);
    expect(payload.raw_text).toBe("Short search snippet");
  });
});
