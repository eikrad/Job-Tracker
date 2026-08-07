import { describe, expect, it, vi } from "vitest";
import type { Job } from "../types";
import { filterJobsByHiddenStatuses } from "./filterJobs";
import {
  loadHiddenJobStatuses,
  normalizeHiddenJobStatuses,
  saveHiddenJobStatuses,
  toggleHiddenJobStatus,
} from "./hiddenJobStatuses";

function job(partial: Partial<Job> & Pick<Job, "id" | "company" | "status">): Job {
  return {
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...partial,
  };
}

describe("hiddenJobStatuses", () => {
  const pipeline = ["Interesting", "Plan to Apply", "Application Sent", "Feedback", "Done"];

  it("drops unknown status names and keeps pipeline order", () => {
    expect(normalizeHiddenJobStatuses(["Done", "Nope", "Interesting"], pipeline)).toEqual([
      "Interesting",
      "Done",
    ]);
    expect(normalizeHiddenJobStatuses([], pipeline)).toEqual([]);
    expect(normalizeHiddenJobStatuses(null, pipeline)).toEqual([]);
  });

  it("toggles statuses on and off", () => {
    expect(toggleHiddenJobStatus([], "Done", pipeline)).toEqual(["Done"]);
    expect(toggleHiddenJobStatus(["Done"], "Done", pipeline)).toEqual([]);
    expect(toggleHiddenJobStatus(["Done"], "Interesting", pipeline)).toEqual([
      "Interesting",
      "Done",
    ]);
  });

  it("loads defaults when localStorage is empty", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: vi.fn(),
    });
    expect(loadHiddenJobStatuses(pipeline)).toEqual([]);
  });

  it("loads and saves hidden statuses", () => {
    const setItem = vi.fn();
    vi.stubGlobal("localStorage", {
      getItem: () => JSON.stringify(["Done", "Interesting"]),
      setItem,
    });
    expect(loadHiddenJobStatuses(pipeline)).toEqual(["Interesting", "Done"]);
    saveHiddenJobStatuses(["Done"]);
    expect(setItem).toHaveBeenCalledWith(
      "jobtracker.hiddenJobStatuses",
      JSON.stringify(["Done"]),
    );
  });
});

describe("filterJobsByHiddenStatuses", () => {
  const jobs = [
    job({ id: 1, company: "A", status: "Interesting" }),
    job({ id: 2, company: "B", status: "Done" }),
    job({ id: 3, company: "C", status: "Feedback" }),
  ];

  it("returns all jobs when nothing is hidden", () => {
    expect(filterJobsByHiddenStatuses(jobs, [])).toEqual(jobs);
  });

  it("removes jobs whose status is hidden", () => {
    expect(filterJobsByHiddenStatuses(jobs, ["Done", "Interesting"]).map((j) => j.id)).toEqual([
      3,
    ]);
  });
});
