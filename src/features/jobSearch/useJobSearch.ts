import { useState, useEffect, useCallback } from "react";
import {
  getKeywordStats,
  getLocationSuggestions,
  fetchJobSearchRss,
  buildSearchUrl,
  openUrlInBrowser,
  type KeywordStat,
  type JobSearchResult,
} from "../../lib/tauriApi";

export type { KeywordStat, JobSearchResult };

export type Platform = "jobindex" | "indeed" | "linkedin";

export const INDEED_REGIONS: { code: string; label: string }[] = [
  { code: "dk", label: "Denmark (dk.indeed.com)" },
  { code: "de", label: "Germany (de.indeed.com)" },
  { code: "se", label: "Sweden (se.indeed.com)" },
  { code: "no", label: "Norway (no.indeed.com)" },
  { code: "fi", label: "Finland (fi.indeed.com)" },
  { code: "com", label: "International (indeed.com)" },
];

const ALL_PLATFORMS: Platform[] = ["jobindex", "indeed", "linkedin"];

type PlatformRecord<T> = Record<Platform, T>;

function makePlatformRecord<T>(value: T): PlatformRecord<T> {
  return { jobindex: value, indeed: value, linkedin: value } as PlatformRecord<T>;
}

export function useJobSearch() {
  // ── Keyword state ──────────────────────────────────────────────────────────
  const [allKeywords, setAllKeywords] = useState<KeywordStat[]>([]);
  const [selectedKeywords, setSelectedKeywords] = useState<Set<string>>(new Set());
  const [customKeyword, setCustomKeyword] = useState("");

  // ── Location state ─────────────────────────────────────────────────────────
  const [locationSuggestions, setLocationSuggestions] = useState<string[]>([]);
  const [location, setLocation] = useState("");

  // ── Platform / settings state ──────────────────────────────────────────────
  const [indeedRegion, setIndeedRegion] = useState("dk");
  const [activePlatforms, setActivePlatforms] = useState<Set<Platform>>(
    new Set(ALL_PLATFORMS),
  );

  // ── Search result state ────────────────────────────────────────────────────
  const [results, setResults] = useState<PlatformRecord<JobSearchResult[]>>(
    makePlatformRecord([]),
  );
  const [loading, setLoading] = useState<PlatformRecord<boolean>>(
    makePlatformRecord(false),
  );
  const [errors, setErrors] = useState<PlatformRecord<string>>(makePlatformRecord(""));
  const [hasSearched, setHasSearched] = useState(false);
  const [linkedinOpened, setLinkedinOpened] = useState(false);

  // ── Load on mount ──────────────────────────────────────────────────────────
  useEffect(() => {
    getKeywordStats()
      .then((stats) => {
        setAllKeywords(stats);
        // Pre-select the top 5 by frequency
        setSelectedKeywords(new Set(stats.slice(0, 5).map((s) => s.keyword)));
      })
      .catch(console.error);

    getLocationSuggestions()
      .then((cities) => {
        setLocationSuggestions(cities);
        // Auto-suggest the most-used city
        if (cities.length > 0) setLocation(cities[0]);
      })
      .catch(console.error);
  }, []);

  // ── Keyword helpers ────────────────────────────────────────────────────────
  const toggleKeyword = useCallback((keyword: string) => {
    setSelectedKeywords((prev) => {
      const next = new Set(prev);
      next.has(keyword) ? next.delete(keyword) : next.add(keyword);
      return next;
    });
  }, []);

  const addCustomKeyword = useCallback(() => {
    const kw = customKeyword.trim().toLowerCase();
    if (!kw) return;
    setAllKeywords((prev) => {
      if (prev.some((s) => s.keyword === kw)) return prev;
      return [...prev, { keyword: kw, count: 0 }];
    });
    setSelectedKeywords((prev) => new Set([...prev, kw]));
    setCustomKeyword("");
  }, [customKeyword]);

  // Remove a custom keyword (count === 0) entirely.
  const removeCustomKeyword = useCallback((keyword: string) => {
    setAllKeywords((prev) => prev.filter((s) => s.keyword !== keyword || s.count > 0));
    setSelectedKeywords((prev) => {
      const next = new Set(prev);
      next.delete(keyword);
      return next;
    });
  }, []);

  // ── Platform toggle ────────────────────────────────────────────────────────
  const togglePlatform = useCallback((platform: Platform) => {
    setActivePlatforms((prev) => {
      const next = new Set(prev);
      next.has(platform) ? next.delete(platform) : next.add(platform);
      return next;
    });
  }, []);

  // ── Open in browser ────────────────────────────────────────────────────────
  const openInBrowser = useCallback(
    async (platform: Platform) => {
      try {
        const url = await buildSearchUrl({
          platform,
          keywords: Array.from(selectedKeywords),
          location: location || null,
          region: platform === "indeed" ? indeedRegion : null,
        });
        await openUrlInBrowser(url);
      } catch (e) {
        console.error("openInBrowser failed:", e);
      }
    },
    [selectedKeywords, location, indeedRegion],
  );

  // ── Main search ────────────────────────────────────────────────────────────
  const search = useCallback(async () => {
    const keywords = Array.from(selectedKeywords);
    if (keywords.length === 0) return;

    setHasSearched(true);
    setLinkedinOpened(false);

    // RSS-capable platforms
    const rssPlats = (["jobindex", "indeed"] as Platform[]).filter((p) =>
      activePlatforms.has(p),
    );

    // Fire all RSS fetches in parallel
    const fetches = rssPlats.map(async (platform) => {
      setLoading((prev) => ({ ...prev, [platform]: true }));
      setErrors((prev) => ({ ...prev, [platform]: "" }));
      try {
        const data = await fetchJobSearchRss({
          platform,
          keywords,
          location: location || null,
          region: platform === "indeed" ? indeedRegion : null,
        });
        setResults((prev) => ({ ...prev, [platform]: data }));
      } catch (e) {
        setErrors((prev) => ({
          ...prev,
          [platform]: e instanceof Error ? e.message : String(e),
        }));
      } finally {
        setLoading((prev) => ({ ...prev, [platform]: false }));
      }
    });

    // LinkedIn: open browser immediately
    if (activePlatforms.has("linkedin")) {
      fetches.push(
        openInBrowser("linkedin").then(() => setLinkedinOpened(true)),
      );
    }

    await Promise.allSettled(fetches);
  }, [selectedKeywords, activePlatforms, location, indeedRegion, openInBrowser]);

  return {
    // Keywords
    allKeywords,
    selectedKeywords,
    customKeyword,
    setCustomKeyword,
    toggleKeyword,
    addCustomKeyword,
    removeCustomKeyword,
    // Location
    locationSuggestions,
    location,
    setLocation,
    // Platforms
    indeedRegion,
    setIndeedRegion,
    activePlatforms,
    togglePlatform,
    // Results
    results,
    loading,
    errors,
    hasSearched,
    linkedinOpened,
    // Actions
    search,
    openInBrowser,
  };
}
