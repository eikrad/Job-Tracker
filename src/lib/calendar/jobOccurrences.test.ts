import { describe, expect, it } from "vitest";
import type { Job } from "../types";
import {
  dateKeyFromParts,
  jobsToOccurrences,
  monthGridDays,
  occurrencesByDateKey,
} from "./jobOccurrences";

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

describe("jobsToOccurrences", () => {
  it("flattens three date kinds", () => {
    const occ = jobsToOccurrences([
      job({
        id: 1,
        company: "A",
        deadline: "2026-06-10",
        interview_date: "2026-06-15",
        start_date: "2026-07-01",
      }),
    ]);
    expect(occ).toHaveLength(3);
    expect(occ.map((o) => o.kind)).toEqual(["apply", "interview", "start"]);
  });

  it("sorts by date then company", () => {
    const occ = jobsToOccurrences([
      job({ id: 2, company: "Zed", deadline: "2026-06-02" }),
      job({ id: 1, company: "Acme", deadline: "2026-06-01" }),
    ]);
    expect(occ[0].company).toBe("Acme");
    expect(occ[1].company).toBe("Zed");
  });
});

describe("occurrencesByDateKey", () => {
  it("filters to month", () => {
    const occ = jobsToOccurrences([
      job({ id: 1, company: "X", deadline: "2026-05-31" }),
      job({ id: 2, company: "Y", deadline: "2026-06-01" }),
    ]);
    const map = occurrencesByDateKey(occ, 2026, 5);
    expect(map.has("2026-05-31")).toBe(false);
    expect(map.has("2026-06-01")).toBe(true);
  });
});

describe("monthGridDays", () => {
  it("June 2026: 1st is Monday (first cell is day 1)", () => {
    const grid = monthGridDays(2026, 5);
    expect(grid[0][0]).toBe(1);
  });
});

describe("dateKeyFromParts", () => {
  it("zero-pads month and day", () => {
    expect(dateKeyFromParts(2026, 2, 5)).toBe("2026-03-05");
  });
});
