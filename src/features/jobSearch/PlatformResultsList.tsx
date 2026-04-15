import { en } from "../../i18n/en";
import { SearchResultCard } from "./SearchResultCard";
import type { JobSearchResult, Platform } from "./useJobSearch";

const PLATFORM_LABELS: Record<Platform, string> = {
  jobindex: "Jobindex.dk",
  indeed: "Indeed",
  linkedin: "LinkedIn",
};

interface Props {
  platform: Platform;
  isActive: boolean;
  results: JobSearchResult[];
  loading: boolean;
  error: string;
  linkedinOpened?: boolean;
  onOpenInBrowser: (platform: Platform) => void;
}

export function PlatformResultsList({
  platform,
  isActive,
  results,
  loading,
  error,
  linkedinOpened,
  onOpenInBrowser,
}: Props) {
  if (!isActive) return null;

  const label = PLATFORM_LABELS[platform];
  const isLinkedIn = platform === "linkedin";

  return (
    <section className="platformSection">
      <div className="platformHeader">
        <h2 className="platformTitle">
          {label}
          {!isLinkedIn && !loading && !error && results.length > 0 && (
            <span className="platformCount">
              {en.jobSearch.resultsCount(results.length)}
            </span>
          )}
        </h2>
        {!isLinkedIn && (
          <button
            type="button"
            className="btn btnGhost btnSm"
            onClick={() => onOpenInBrowser(platform)}
          >
            {en.jobSearch.openInBrowser}
          </button>
        )}
      </div>

      {isLinkedIn ? (
        <div className="platformBrowserOnly card">
          <p className="muted">{en.jobSearch.linkedinBrowserOnly}</p>
          {linkedinOpened ? (
            <p className="platformLinkedinOpened">{en.jobSearch.linkedinOpened}</p>
          ) : (
            <button
              type="button"
              className="btn btnPrimary"
              onClick={() => onOpenInBrowser("linkedin")}
            >
              {en.jobSearch.openLinkedIn}
            </button>
          )}
        </div>
      ) : loading ? (
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
          <p className="muted">{en.jobSearch.noResults}</p>
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
          {results.map((result, i) => (
            <SearchResultCard key={`${result.url}-${i}`} result={result} />
          ))}
        </div>
      )}
    </section>
  );
}
