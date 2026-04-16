import { en } from "../../i18n/en";
import { SearchResultCard } from "./SearchResultCard";
import type { JobSearchResult, Platform } from "./useJobSearch";
import type { JobSearchFallbackHint } from "../../lib/tauriApi";

const PLATFORM_LABELS: Record<Platform, string> = {
  jobindex: "Jobindex.dk",
  indeed: "Indeed",
  linkedin: "LinkedIn",
  thehub: "The Hub",
};

interface Props {
  platform: Platform;
  isActive: boolean;
  hidden?: boolean;
  results: JobSearchResult[];
  loading: boolean;
  error: string;
  fallbackHint?: JobSearchFallbackHint | null;
  onOpenInBrowser: (platform: Platform) => void;
}

function resultKey(result: JobSearchResult): string {
  if (result.url) return `${result.platform}|${result.url}`;
  return [
    result.platform,
    result.title,
    result.company,
    result.location,
    result.published_date,
  ].join("|");
}

export function PlatformResultsList({
  platform,
  isActive,
  hidden = false,
  results,
  loading,
  error,
  fallbackHint,
  onOpenInBrowser,
}: Props) {
  if (!isActive || hidden) return null;

  const label = PLATFORM_LABELS[platform];

  return (
    <section className="platformSection">
      <div className="platformHeader">
        <h2 className="platformTitle">
          {label}
          {!loading && !error && results.length > 0 && (
            <span className="platformCount">
              {en.jobSearch.resultsCount(results.length)}
            </span>
          )}
        </h2>
        <button
          type="button"
          className="btn btnGhost btnSm"
          onClick={() => onOpenInBrowser(platform)}
        >
          {en.jobSearch.openInBrowser}
        </button>
      </div>

      {loading ? (
        <div className="platformLoading">
          <span className="muted">{en.jobSearch.loading}</span>
        </div>
      ) : error ? (
        <div className="platformError card">
          <p className="muted">
            {en.jobSearch.fetchError}: <code>{error}</code>
          </p>
          <button
            type="button"
            className="btn btnGhost"
            onClick={() => onOpenInBrowser(platform)}
          >
            {en.jobSearch.tryInBrowser}
          </button>
        </div>
      ) : results.length === 0 ? (
        <div className="platformEmpty">
          <p className="muted">
            {fallbackHint?.reason || en.jobSearch.noResults}
          </p>
          <button
            type="button"
            className="btn btnGhost btnSm"
            onClick={() => onOpenInBrowser(platform)}
          >
            {en.jobSearch.tryInBrowser}
          </button>
        </div>
      ) : (
        <div className="searchResultList">
          {results.map((result) => (
            <SearchResultCard key={resultKey(result)} result={result} />
          ))}
        </div>
      )}
    </section>
  );
}
