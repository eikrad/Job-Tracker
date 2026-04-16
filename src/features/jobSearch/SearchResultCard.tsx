import { memo, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { fetchJobSearchResultPageText, openUrlInBrowser } from "../../lib/tauriApi";
import { en } from "../../i18n/en";
import { useJobTracker } from "../../context/JobTrackerContext";
import type { JobSearchResult } from "./useJobSearch";
import { buildSavedJobPayload } from "./saveSearchResult";

interface Props {
  result: JobSearchResult;
}

function formatRelativeDate(dateStr: string): string {
  if (!dateStr) return "";
  try {
    const d = new Date(dateStr);
    if (Number.isNaN(d.getTime())) return dateStr;
    const diffMs = Date.now() - d.getTime();
    const diffDays = Math.floor(diffMs / 86_400_000);
    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays}d ago`;
    if (diffDays < 31) return `${Math.floor(diffDays / 7)}w ago`;
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  } catch {
    return dateStr;
  }
}

function normalizeScore(value?: number): number {
  if (typeof value !== "number" || Number.isNaN(value)) return 0;
  return Math.max(0, Math.min(1, value));
}

function scoreToneClass(totalScore?: number): string {
  const score = normalizeScore(totalScore);
  if (score >= 0.75) return "searchScoreToneHigh";
  if (score >= 0.45) return "searchScoreToneMedium";
  return "searchScoreToneLow";
}

export const SearchResultCard = memo(function SearchResultCard({ result }: Props) {
  const navigate = useNavigate();
  const { onSubmit } = useJobTracker();
  const [isSaving, setIsSaving] = useState(false);
  const [added, setAdded] = useState(false);

  function handleOpenBrowser() {
    openUrlInBrowser(result.url).catch(console.error);
  }

  async function handleAddToTracker() {
    if (isSaving || added) return;
    setIsSaving(true);
    try {
      const payload = await buildSavedJobPayload(result, fetchJobSearchResultPageText);
      const ok = await onSubmit(payload);
      if (ok) setAdded(true);
    } catch (error) {
      console.error("Failed to add job:", error);
    } finally {
      setIsSaving(false);
    }
  }

  function handleOpenFullForm() {
    navigate("/jobs/new", {
      state: { prefillUrl: result.url },
    });
  }

  const metaParts = useMemo(
    () => [result.company, result.location].filter(Boolean),
    [result.company, result.location],
  );
  const dateLabel = useMemo(
    () => formatRelativeDate(result.published_date),
    [result.published_date],
  );
  const scoreLabel = useMemo(
    () =>
      en.jobSearch.scoreBreakdown(
        normalizeScore(result.freshness_score),
        normalizeScore(result.keyword_score),
        normalizeScore(result.total_score),
      ),
    [result.freshness_score, result.keyword_score, result.total_score],
  );
  const scoreTone = useMemo(
    () => scoreToneClass(result.total_score),
    [result.total_score],
  );

  return (
    <article className="searchResultCard">
      <div className="searchResultMain">
        <h3 className="searchResultTitle">{result.title || en.common.untitled}</h3>
        {(metaParts.length > 0 || dateLabel) && (
          <p className="searchResultMeta">
            {metaParts.join(" · ")}
            {metaParts.length > 0 && dateLabel && (
              <span className="searchResultSep"> · </span>
            )}
            {dateLabel && <span className="searchResultDate">{dateLabel}</span>}
          </p>
        )}
        {result.description && (
          <p className="searchResultDesc">{result.description}</p>
        )}
        <p className={`searchScoreBadge ${scoreTone}`}>{scoreLabel}</p>
      </div>

      <div className="searchResultActions">
        <button type="button" className="btn btnGhost btnSm" onClick={handleOpenBrowser}>
          {en.jobSearch.openInBrowser}
        </button>
        <button
          type="button"
          className="btn btnPrimary btnSm"
          onClick={() => void handleAddToTracker()}
          disabled={isSaving || added}
        >
          {added
            ? en.jobSearch.addedToTracker
            : isSaving
              ? en.jobSearch.addingToTracker
              : en.jobSearch.addAsInteresting}
        </button>
        <button type="button" className="btn btnGhost btnSm" onClick={handleOpenFullForm}>
          {en.jobSearch.openForm}
        </button>
      </div>
    </article>
  );
});
