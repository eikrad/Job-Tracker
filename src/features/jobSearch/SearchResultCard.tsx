import { useNavigate } from "react-router-dom";
import { openUrlInBrowser } from "../../lib/tauriApi";
import { en } from "../../i18n/en";
import type { JobSearchResult } from "./useJobSearch";

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

export function SearchResultCard({ result }: Props) {
  const navigate = useNavigate();

  function handleOpenBrowser() {
    openUrlInBrowser(result.url).catch(console.error);
  }

  function handleAddToTracker() {
    navigate("/jobs/new", {
      state: { prefillUrl: result.url },
    });
  }

  const metaParts = [result.company, result.location].filter(Boolean);
  const dateLabel = formatRelativeDate(result.published_date);

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
      </div>

      <div className="searchResultActions">
        <button type="button" className="btn btnGhost btnSm" onClick={handleOpenBrowser}>
          {en.jobSearch.openInBrowser}
        </button>
        <button type="button" className="btn btnPrimary btnSm" onClick={handleAddToTracker}>
          {en.jobSearch.addToTracker}
        </button>
      </div>
    </article>
  );
}
