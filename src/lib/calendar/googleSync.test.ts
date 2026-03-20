import { describe, expect, it } from "vitest";
import type { Job } from "../types";
import { googleCalendarEventLink, googleCalendarTemplateUrl } from "./googleSync";

function job(partial: Partial<Job> & Pick<Job, "id" | "company">): Job {
  return {
    id: partial.id,
    company: partial.company,
    title: partial.title ?? null,
    url: partial.url ?? null,
    raw_text: partial.raw_text ?? null,
    status: partial.status ?? "Interesting",
    deadline: partial.deadline ?? null,
    interview_date: partial.interview_date ?? null,
    start_date: partial.start_date ?? null,
    tags: partial.tags ?? null,
    detected_language: partial.detected_language ?? null,
    notes: partial.notes ?? null,
    pdf_path: partial.pdf_path ?? null,
    created_at: partial.created_at ?? "2026-01-01T00:00:00Z",
    updated_at: partial.updated_at ?? "2026-01-01T00:00:00Z",
  };
}

describe("googleCalendarTemplateUrl", () => {
  it("builds URL for arbitrary date", () => {
    const u = googleCalendarTemplateUrl({
      date: "2026-04-01",
      summaryLine: "Interview: Acme",
      details: "Dev",
    });
    expect(u).toContain("calendar.google.com");
    expect(u).toContain("20260401/20260401");
  });
});

describe("googleCalendarEventLink", () => {
  it("returns null without deadline", () => {
    expect(googleCalendarEventLink(job({ id: 1, company: "A" }))).toBeNull();
  });

  it("builds template URL with YYYYMMDD dates and encoded title", () => {
    const url = googleCalendarEventLink(
      job({ id: 2, company: "Mærsk", title: "Data", deadline: "2026-03-19" }),
    );
    expect(url).toContain("calendar.google.com");
    expect(url).toContain("action=TEMPLATE");
    expect(url).toContain("20260319/20260319");
    expect(url).toContain(encodeURIComponent("Application Deadline: Mærsk"));
    expect(url).toContain(encodeURIComponent("Data"));
  });
});
