import { describe, expect, it } from "vitest";
import type { Job, NewJob } from "../types";
import { findDuplicateJob } from "./duplicateCheck";

const baseJob = (over: Partial<Job>): Job => ({
  id: 1,
  company: "Acme",
  title: "Dev",
  url: null,
  raw_text: null,
  status: "Interesting",
  deadline: null,
  tags: null,
  detected_language: null,
  notes: null,
  pdf_path: null,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  ...over,
});

describe("findDuplicateJob", () => {
  it("matches same URL", () => {
    const jobs = [baseJob({ id: 1, url: "https://jobs.example/1" })];
    const payload: NewJob = {
      company: "Other",
      status: "Interesting",
      url: "https://jobs.example/1",
    };
    expect(findDuplicateJob(jobs, payload)).toBe(jobs[0]);
  });

  it("matches same company and title case-insensitive", () => {
    const jobs = [baseJob({ id: 2, company: "Beta", title: "Engineer" })];
    const payload: NewJob = {
      company: "beta",
      title: "engineer",
      status: "Interesting",
    };
    expect(findDuplicateJob(jobs, payload)).toBe(jobs[0]);
  });

  it("returns undefined when no match", () => {
    const jobs = [baseJob({ id: 3 })];
    const payload: NewJob = { company: "Gamma", title: "Other", status: "Interesting" };
    expect(findDuplicateJob(jobs, payload)).toBeUndefined();
  });
});
