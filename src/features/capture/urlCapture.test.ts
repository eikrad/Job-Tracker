import { describe, expect, it } from "vitest";
import { createBaseCaptureDraft, normalizeCaptureUrl, sourceFromUrl } from "./urlCapture";

describe("normalizeCaptureUrl", () => {
  it("accepts valid http/https URLs", () => {
    expect(normalizeCaptureUrl("https://example.com/job")).toBe("https://example.com/job");
    expect(normalizeCaptureUrl("http://example.com")).toBe("http://example.com/");
  });

  it("rejects invalid and non-http URLs", () => {
    expect(normalizeCaptureUrl("")).toBeNull();
    expect(normalizeCaptureUrl("example.com/job")).toBeNull();
    expect(normalizeCaptureUrl("ftp://example.com/job")).toBeNull();
  });
});

describe("sourceFromUrl", () => {
  it("normalizes hostname and strips www", () => {
    expect(sourceFromUrl("https://www.linkedin.com/jobs/view/123")).toBe("linkedin.com");
    expect(sourceFromUrl("https://jobs.thehub.io/role")).toBe("jobs.thehub.io");
  });

  it("returns undefined for invalid URLs", () => {
    expect(sourceFromUrl("not-url")).toBeUndefined();
  });
});

describe("createBaseCaptureDraft", () => {
  it("creates a draft with required capture defaults", () => {
    expect(createBaseCaptureDraft("https://example.com/job", "Interesting")).toEqual({
      company: "",
      status: "Interesting",
      url: "https://example.com/job",
      source: "example.com",
    });
  });
});
