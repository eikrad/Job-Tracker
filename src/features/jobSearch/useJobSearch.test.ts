// @vitest-environment happy-dom
import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  buildSearchUrl,
  fetchJobSearchResults,
  getKeywordStats,
  getLocationSuggestions,
  openUrlInBrowser,
} from "../../lib/tauriApi";
import { useJobSearch } from "./useJobSearch";

// ── Mock the entire Tauri API layer ────────────────────────────────────────
vi.mock("../../lib/tauriApi", () => ({
  getKeywordStats: vi.fn(),
  getLocationSuggestions: vi.fn(),
  fetchJobSearchResults: vi.fn(),
  buildSearchUrl: vi.fn(),
  openUrlInBrowser: vi.fn(),
}));

// ── Mock JobTrackerContext so the hook doesn't need a provider ─────────────
vi.mock("../../context/JobTrackerContext", () => ({
  useJobTracker: () => ({
    serpApiKey: "",
    braveSearchApiKey: "",
    onSubmit: vi.fn(),
  }),
}));

const mockKeywords = [
  { keyword: "react", count: 10 },
  { keyword: "typescript", count: 8 },
  { keyword: "node.js", count: 6 },
  { keyword: "python", count: 4 },
  { keyword: "docker", count: 2 },
  { keyword: "java", count: 1 }, // 6th — should NOT be pre-selected
];

const mockCities = ["Copenhagen", "Aarhus"];

const mockResult = {
  title: "Dev Job",
  company: "Acme",
  location: "Copenhagen",
  url: "https://jobindex.dk/job/1",
  description: "Great job",
  published_date: "2026-04-14",
  platform: "jobindex",
};

beforeEach(() => {
  vi.mocked(getKeywordStats).mockResolvedValue(mockKeywords);
  vi.mocked(getLocationSuggestions).mockResolvedValue(mockCities);
  vi.mocked(fetchJobSearchResults).mockResolvedValue([]);
  vi.mocked(buildSearchUrl).mockResolvedValue("https://example.com/search");
  vi.mocked(openUrlInBrowser).mockResolvedValue(undefined);
});

afterEach(() => vi.clearAllMocks());

// ── Initial state ──────────────────────────────────────────────────────────

describe("initial state", () => {
  it("activates all three platforms by default", () => {
    const { result } = renderHook(() => useJobSearch());
    expect(result.current.activePlatforms.has("jobindex")).toBe(true);
    expect(result.current.activePlatforms.has("indeed")).toBe(true);
    expect(result.current.activePlatforms.has("linkedin")).toBe(true);
  });

  it("starts with empty keyword and result lists", () => {
    const { result } = renderHook(() => useJobSearch());
    expect(result.current.allKeywords).toHaveLength(0);
    expect(result.current.results.jobindex).toHaveLength(0);
    expect(result.current.hasSearched).toBe(false);
  });

  it("sets indeedRegion to 'dk' by default", () => {
    const { result } = renderHook(() => useJobSearch());
    expect(result.current.indeedRegion).toBe("dk");
  });
});

// ── On-mount data loading ──────────────────────────────────────────────────

describe("on mount", () => {
  it("loads keyword stats and pre-selects top 5", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords).toHaveLength(6));
    expect(result.current.selectedKeywords.size).toBe(5);
    expect(result.current.selectedKeywords.has("react")).toBe(true);
    expect(result.current.selectedKeywords.has("typescript")).toBe(true);
    expect(result.current.selectedKeywords.has("java")).toBe(false); // 6th → not selected
  });

  it("loads location suggestions and auto-fills the first city", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.location).toBe("Copenhagen"));
    expect(result.current.locationSuggestions).toEqual(mockCities);
  });

  it("handles getKeywordStats failure gracefully (no crash)", async () => {
    vi.mocked(getKeywordStats).mockRejectedValueOnce(new Error("DB offline"));
    const { result } = renderHook(() => useJobSearch());
    // Should not throw; state stays empty
    await act(async () => {});
    expect(result.current.allKeywords).toHaveLength(0);
  });
});

// ── Keyword management ─────────────────────────────────────────────────────

describe("toggleKeyword", () => {
  it("deselects a selected keyword", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.selectedKeywords.has("react")).toBe(true));
    act(() => result.current.toggleKeyword("react"));
    expect(result.current.selectedKeywords.has("react")).toBe(false);
  });

  it("re-selects a deselected keyword", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.selectedKeywords.has("react")).toBe(true));
    act(() => result.current.toggleKeyword("react"));
    act(() => result.current.toggleKeyword("react"));
    expect(result.current.selectedKeywords.has("react")).toBe(true);
  });
});

describe("addCustomKeyword", () => {
  it("normalises to lowercase, adds to list and selection, clears input", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    act(() => result.current.setCustomKeyword("GraphQL"));
    act(() => result.current.addCustomKeyword());

    expect(result.current.allKeywords.some((k) => k.keyword === "graphql")).toBe(true);
    expect(result.current.selectedKeywords.has("graphql")).toBe(true);
    expect(result.current.customKeyword).toBe("");
  });

  it("is a no-op for blank input", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));
    const countBefore = result.current.allKeywords.length;

    act(() => result.current.setCustomKeyword("   "));
    act(() => result.current.addCustomKeyword());

    expect(result.current.allKeywords.length).toBe(countBefore);
  });

  it("does not add a duplicate keyword already in the list", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));
    const countBefore = result.current.allKeywords.length;

    act(() => result.current.setCustomKeyword("React")); // "react" already exists
    act(() => result.current.addCustomKeyword());

    expect(result.current.allKeywords.length).toBe(countBefore);
  });
});

describe("removeCustomKeyword", () => {
  it("removes a count=0 keyword from the list and selection", async () => {
    const { result } = renderHook(() => useJobSearch());

    act(() => result.current.setCustomKeyword("custom-kw"));
    act(() => result.current.addCustomKeyword());
    expect(result.current.selectedKeywords.has("custom-kw")).toBe(true);

    act(() => result.current.removeCustomKeyword("custom-kw"));
    expect(result.current.allKeywords.some((k) => k.keyword === "custom-kw")).toBe(false);
    expect(result.current.selectedKeywords.has("custom-kw")).toBe(false);
  });

  it("does not remove a keyword that has a positive count", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    act(() => result.current.removeCustomKeyword("react")); // count=10, should stay
    expect(result.current.allKeywords.some((k) => k.keyword === "react")).toBe(true);
  });
});

// ── Platform management ────────────────────────────────────────────────────

describe("togglePlatform", () => {
  it("disables an active platform", () => {
    const { result } = renderHook(() => useJobSearch());
    act(() => result.current.togglePlatform("jobindex"));
    expect(result.current.activePlatforms.has("jobindex")).toBe(false);
  });

  it("re-enables a disabled platform", () => {
    const { result } = renderHook(() => useJobSearch());
    act(() => result.current.togglePlatform("jobindex"));
    act(() => result.current.togglePlatform("jobindex"));
    expect(result.current.activePlatforms.has("jobindex")).toBe(true);
  });
});

// ── Search behaviour ───────────────────────────────────────────────────────

describe("search", () => {
  it("is a no-op when no keywords are selected", async () => {
    vi.mocked(getKeywordStats).mockResolvedValueOnce([]);
    const { result } = renderHook(() => useJobSearch());
    await act(async () => {
      await result.current.search();
    });
    expect(fetchJobSearchResults).not.toHaveBeenCalled();
    expect(result.current.hasSearched).toBe(false);
  });

  it("calls fetchJobSearchResults for jobindex and indeed when active", async () => {
    vi.mocked(fetchJobSearchResults).mockResolvedValue([mockResult]);
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    await act(async () => {
      await result.current.search();
    });

    const platforms = vi.mocked(fetchJobSearchResults).mock.calls.map(
      (c) => c[0].platform,
    );
    expect(platforms).toContain("jobindex");
    expect(platforms).toContain("indeed");
    expect(result.current.hasSearched).toBe(true);
    expect(result.current.results.jobindex).toHaveLength(1);
  });

  it("does NOT call fetchJobSearchResults for LinkedIn", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    await act(async () => {
      await result.current.search();
    });

    const platforms = vi.mocked(fetchJobSearchResults).mock.calls.map(
      (c) => c[0].platform,
    );
    expect(platforms).not.toContain("linkedin");
  });

  it("opens the browser for LinkedIn", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    await act(async () => {
      await result.current.search();
    });

    expect(buildSearchUrl).toHaveBeenCalledWith(
      expect.objectContaining({ platform: "linkedin" }),
    );
    expect(openUrlInBrowser).toHaveBeenCalled();
    expect(result.current.linkedinOpened).toBe(true);
  });

  it("sets per-platform error state when fetch fails", async () => {
    vi.mocked(fetchJobSearchResults).mockRejectedValue(new Error("API blocked"));
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    await act(async () => {
      await result.current.search();
    });

    expect(result.current.errors.jobindex).toContain("API blocked");
    expect(result.current.loading.jobindex).toBe(false);
  });

  it("resets loading to false after fetch (success)", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    await act(async () => {
      await result.current.search();
    });

    expect(result.current.loading.jobindex).toBe(false);
    expect(result.current.loading.indeed).toBe(false);
  });

  it("skips a platform that has been toggled off", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    act(() => result.current.togglePlatform("indeed")); // disable Indeed

    await act(async () => {
      await result.current.search();
    });

    const platforms = vi.mocked(fetchJobSearchResults).mock.calls.map(
      (c) => c[0].platform,
    );
    expect(platforms).not.toContain("indeed");
  });

  it("passes serpApiKey and braveSearchApiKey to fetchJobSearchResults", async () => {
    const { result } = renderHook(() => useJobSearch());
    await waitFor(() => expect(result.current.allKeywords.length).toBeGreaterThan(0));

    // Toggle off linkedin and indeed to simplify
    act(() => result.current.togglePlatform("indeed"));
    act(() => result.current.togglePlatform("linkedin"));

    await act(async () => {
      await result.current.search();
    });

    expect(fetchJobSearchResults).toHaveBeenCalledWith(
      expect.objectContaining({
        platform: "jobindex",
        serpApiKey: null,
        braveSearchApiKey: null,
      }),
    );
  });
});
