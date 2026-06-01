import { describe, expect, it, vi } from "vitest";
import {
  DEFAULT_VISIBLE_JOB_TABLE_COLUMNS,
  loadVisibleJobTableColumns,
  normalizeVisibleColumns,
  toggleVisibleColumn,
} from "./jobTableColumns";

describe("jobTableColumns", () => {
  it("normalizes invalid stored values to defaults", () => {
    expect(normalizeVisibleColumns(["company", "not-a-column"])).toEqual(["company"]);
    expect(normalizeVisibleColumns([])).toEqual([...DEFAULT_VISIBLE_JOB_TABLE_COLUMNS]);
  });

  it("keeps at least one visible column when toggling off", () => {
    expect(toggleVisibleColumn(["company"], "company")).toEqual(["company"]);
  });

  it("adds columns in canonical order", () => {
    expect(toggleVisibleColumn(["company", "title"], "deadline")).toEqual([
      "company",
      "title",
      "deadline",
    ]);
  });

  it("loads defaults when localStorage is empty", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: vi.fn(),
    });
    expect(loadVisibleJobTableColumns()).toEqual([...DEFAULT_VISIBLE_JOB_TABLE_COLUMNS]);
  });
});
