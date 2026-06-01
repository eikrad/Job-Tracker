import { describe, expect, it } from "vitest";
import type { Job } from "../types";
import { sortJobs } from "./sortJobs";

const statuses = ["Interesting", "Plan to Apply", "Application Sent", "Feedback", "Done"];

function job(partial: Partial<Job> & Pick<Job, "id" | "company">): Job {
  return {
    title: null,
    url: null,
    raw_text: null,
    status: "Interesting",
    deadline: null,
    interview_date: null,
    start_date: null,
    tags: null,
    detected_language: null,
    notes: null,
    contact_name: null,
    contact_email: null,
    contact_phone: null,
    workplace_street: null,
    workplace_city: null,
    workplace_postal_code: null,
    work_mode: null,
    salary_range: null,
    contract_type: null,
    priority: null,
    reference_number: null,
    source: null,
    pdf_path: null,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...partial,
  };
}

describe("sortJobs", () => {
  it("sorts by status order then company", () => {
    const jobs = [
      job({ id: 1, company: "Zeta", status: "Application Sent" }),
      job({ id: 2, company: "Alpha", status: "Interesting" }),
      job({ id: 3, company: "Beta", status: "Interesting" }),
    ];
    const sorted = sortJobs(jobs, {
      primary: "status",
      primaryDirection: "asc",
      secondary: "company",
      secondaryDirection: "asc",
      statusOrder: statuses,
    });
    expect(sorted.map((j) => j.company)).toEqual(["Alpha", "Beta", "Zeta"]);
  });

  it("sorts by created_at descending", () => {
    const jobs = [
      job({ id: 1, company: "Old", created_at: "2026-01-01T00:00:00Z" }),
      job({ id: 2, company: "New", created_at: "2026-06-01T00:00:00Z" }),
    ];
    const sorted = sortJobs(jobs, {
      primary: "created_at",
      primaryDirection: "desc",
      secondary: null,
      secondaryDirection: "asc",
    });
    expect(sorted[0].company).toBe("New");
  });
});
