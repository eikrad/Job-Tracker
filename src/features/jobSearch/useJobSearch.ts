import { useState, useEffect, useCallback } from "react";
import {
  getKeywordStats,
  getLocationSuggestions,
  fetchJobSearchResultsBundle,
  buildSearchUrl,
  openUrlInBrowser,
  type KeywordStat,
  type JobSearchResult,
  type JobSearchFallbackHint,
} from "../../lib/tauriApi";
import { useJobTracker } from "../../context/JobTrackerContext";

export type { KeywordStat, JobSearchResult };

export type Platform = "jobindex" | "indeed" | "linkedin" | "thehub";
export type SearchViewMode = "global" | "perPlatform";

export const INDEED_REGIONS: { code: string; label: string }[] = [
  { code: "dk", label: "Denmark (dk.indeed.com)" },
  { code: "de", label: "Germany (de.indeed.com)" },
  { code: "se", label: "Sweden (se.indeed.com)" },
  { code: "no", label: "Norway (no.indeed.com)" },
  { code: "fi", label: "Finland (fi.indeed.com)" },
  { code: "com", label: "International (indeed.com)" },
];

const ALL_PLATFORMS: Platform[] = ["jobindex", "indeed", "linkedin", "thehub"];

type PlatformRecord<T> = Record<Platform, T>;

function makePlatformRecord<T>(value: T): PlatformRecord<T> {
  return {
    jobindex: value,
    indeed: value,
    linkedin: value,
    thehub: value,
  } as PlatformRecord<T>;
}

function getSelectedPlatforms(activePlatforms: Set<Platform>): Platform[] {
  return ALL_PLATFORMS.filter((platform) => activePlatforms.has(platform));
}

function mapPlatformRecord<T>(
  source: Partial<Record<string, T>>,
  fallback: PlatformRecord<T>[Platform],
): PlatformRecord<T> {
  const next = makePlatformRecord(fallback);
  for (const platform of ALL_PLATFORMS) {
    next[platform] = source[platform] ?? fallback;
  }
  return next;
}

export function useJobSearch() {
  const { serpApiKey, braveSearchApiKey } = useJobTracker();
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
  const [globalTop5, setGlobalTop5] = useState<JobSearchResult[]>([]);
  const [viewMode, setViewMode] = useState<SearchViewMode>("global");
  const [fallbackHints, setFallbackHints] = useState<PlatformRecord<JobSearchFallbackHint | null>>(
    makePlatformRecord(null),
  );

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
      if (next.has(keyword)) {
        next.delete(keyword);
      } else {
        next.add(keyword);
      }
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
      if (next.has(platform)) {
        next.delete(platform);
      } else {
        next.add(platform);
      }
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
    const selectedPlatforms = getSelectedPlatforms(activePlatforms);
    setLoading((prev) => {
      const next = { ...prev };
      for (const p of selectedPlatforms) {
        next[p] = true;
      }
      return next;
    });
    setErrors(makePlatformRecord(""));
    setFallbackHints(makePlatformRecord(null));

    try {
      const bundle = await fetchJobSearchResultsBundle({
        keywords,
        location: location || null,
        region: indeedRegion,
        platforms: selectedPlatforms,
        serpApiKey: serpApiKey || null,
        braveSearchApiKey: braveSearchApiKey || null,
      });

      setGlobalTop5(bundle.global_top5);
      setResults(mapPlatformRecord<JobSearchResult[]>(bundle.top5_per_platform, []));
      setFallbackHints(
        mapPlatformRecord<JobSearchFallbackHint | null>(bundle.fallback_hints, null),
      );
    } catch (e) {
      const err = e instanceof Error ? e.message : String(e);
      setErrors((prev) => {
        const next = { ...prev };
        for (const p of selectedPlatforms) {
          next[p] = err;
        }
        return next;
      });
    } finally {
      setLoading((prev) => {
        const next = { ...prev };
        for (const p of selectedPlatforms) {
          next[p] = false;
        }
        return next;
      });
    }
  }, [
    selectedKeywords,
    activePlatforms,
    location,
    indeedRegion,
    serpApiKey,
    braveSearchApiKey,
  ]);

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
    globalTop5,
    viewMode,
    setViewMode,
    fallbackHints,
    loading,
    errors,
    hasSearched,
    // Actions
    search,
    openInBrowser,
  };
}
