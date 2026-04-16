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

  it("excludeJobId skips an otherwise-matching job", () => {
    const jobs = [baseJob({ id: 7, url: "https://jobs.example/7" })];
    const payload: NewJob = { company: "Acme", status: "Interesting", url: "https://jobs.example/7" };
    expect(findDuplicateJob(jobs, payload, 7)).toBeUndefined();
    expect(findDuplicateJob(jobs, payload, 99)).toBe(jobs[0]);
  });

  it("both URLs null do not match by URL", () => {
    const jobs = [baseJob({ id: 4, url: null, company: "Other" })];
    const payload: NewJob = { company: "Acme", status: "Interesting", url: undefined };
    // null/undefined URLs → !!null = false → no URL match; companies differ → no match
    expect(findDuplicateJob(jobs, payload)).toBeUndefined();
  });

  it("empty-string URL in payload does not match", () => {
    const jobs = [baseJob({ id: 5, url: "https://jobs.example/5" })];
    const payload: NewJob = { company: "Other", status: "Interesting", url: "" };
    // !!'' = false → no URL match
    expect(findDuplicateJob(jobs, payload)).toBeUndefined();
  });

  it("both titles null match by company with empty-string comparison", () => {
    // (null ?? '') === (null ?? '') → '' === '' → true when companies also match
    const jobs = [baseJob({ id: 6, title: null })];
    const payload: NewJob = { company: "Acme", title: undefined, status: "Interesting" };
    expect(findDuplicateJob(jobs, payload)).toBe(jobs[0]);
  });

  it("returns the first of multiple matches", () => {
    const jobs = [
      baseJob({ id: 10, url: "https://jobs.example/dup" }),
      baseJob({ id: 11, url: "https://jobs.example/dup" }),
    ];
    const payload: NewJob = { company: "Acme", status: "Interesting", url: "https://jobs.example/dup" };
    expect(findDuplicateJob(jobs, payload)).toBe(jobs[0]);
  });
});
