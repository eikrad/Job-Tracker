import { describe, expect, it, vi } from "vitest";
import {
  loadJobSearchQuery,
  normalizeJobSearchQuery,
  saveJobSearchQuery,
} from "./jobSearchQuery";

describe("jobSearchQuery", () => {
  it("normalizes nullish and trims outer whitespace for storage reads", () => {
    expect(normalizeJobSearchQuery(null)).toBe("");
    expect(normalizeJobSearchQuery(undefined)).toBe("");
    expect(normalizeJobSearchQuery(12)).toBe("");
    expect(normalizeJobSearchQuery("  acme  ")).toBe("acme");
  });

  it("loads an empty string when localStorage is empty", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: vi.fn(),
    });
    expect(loadJobSearchQuery()).toBe("");
  });

  it("loads a stored query", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => "acme",
      setItem: vi.fn(),
    });
    expect(loadJobSearchQuery()).toBe("acme");
  });

  it("saves the query", () => {
    const setItem = vi.fn();
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem,
    });
    saveJobSearchQuery("  beta  ");
    expect(setItem).toHaveBeenCalledWith("jobtracker.jobSearchQuery", "beta");
  });
});
