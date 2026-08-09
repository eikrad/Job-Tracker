import { describe, expect, it } from "vitest";
import type { Job } from "../types";
import { filterJobs, filterJobsBySearch } from "./filterJobs";

function job(partial: Partial<Job> & Pick<Job, "id" | "company">): Job {
  return {
    status: "Interesting",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...partial,
  };
}

describe("filterJobsBySearch", () => {
  const jobs = [
    job({ id: 1, company: "Acme Corp", title: "Frontend Engineer" }),
    job({ id: 2, company: "Beta GmbH", title: "Backend Developer" }),
    job({ id: 3, company: "Gamma", title: null }),
  ];

  it("returns all jobs when the query is empty or whitespace", () => {
    expect(filterJobsBySearch(jobs, "")).toEqual(jobs);
    expect(filterJobsBySearch(jobs, "   ")).toEqual(jobs);
  });

  it("matches company names case-insensitively", () => {
    expect(filterJobsBySearch(jobs, "acme").map((j) => j.id)).toEqual([1]);
  });

  it("matches titles case-insensitively", () => {
    expect(filterJobsBySearch(jobs, "backend").map((j) => j.id)).toEqual([2]);
  });

  it("does not match status or other fields", () => {
    expect(filterJobsBySearch(jobs, "Interesting")).toEqual([]);
  });

  it("treats a missing title as empty", () => {
    expect(filterJobsBySearch(jobs, "gamma").map((j) => j.id)).toEqual([3]);
    expect(filterJobsBySearch(jobs, "engineer").map((j) => j.id)).toEqual([1]);
  });
});

describe("filterJobs", () => {
  const jobs = [
    job({ id: 1, company: "Acme Corp", title: "Frontend Engineer", status: "Interesting" }),
    job({ id: 2, company: "Acme Labs", title: "Backend Developer", status: "Done" }),
    job({ id: 3, company: "Beta", title: "Frontend Lead", status: "Feedback" }),
  ];

  it("combines search and hidden-status filters", () => {
    expect(
      filterJobs(jobs, { query: "acme", hiddenStatuses: ["Done"] }).map((j) => j.id),
    ).toEqual([1]);
  });
});
