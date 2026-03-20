import { describe, expect, it } from "vitest";
import type { Job } from "../types";
import { groupJobsByStatus } from "./groupJobsByStatus";

const job = (id: number, status: string): Job => ({
  id,
  company: `C${id}`,
  title: null,
  url: null,
  raw_text: null,
  status,
  deadline: null,
  tags: null,
  detected_language: null,
  notes: null,
  pdf_path: null,
  created_at: "",
  updated_at: "",
});

describe("groupJobsByStatus", () => {
  it("groups multiple jobs per status", () => {
    const jobs = [job(1, "A"), job(2, "A"), job(3, "B")];
    const map = groupJobsByStatus(jobs);
    expect(map.get("A")).toHaveLength(2);
    expect(map.get("B")).toHaveLength(1);
  });

  it("returns empty map for empty list", () => {
    expect(groupJobsByStatus([]).size).toBe(0);
  });
});
