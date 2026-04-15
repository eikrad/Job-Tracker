import { en } from "../../i18n/en";
import type { KeywordStat } from "./useJobSearch";

interface Props {
  keywords: KeywordStat[];
  selected: Set<string>;
  onToggle: (keyword: string) => void;
  onRemoveCustom: (keyword: string) => void;
  customKeyword: string;
  onCustomChange: (v: string) => void;
  onAddCustom: () => void;
}

export function KeywordPanel({
  keywords,
  selected,
  onToggle,
  onRemoveCustom,
  customKeyword,
  onCustomChange,
  onAddCustom,
}: Props) {
  const maxCount = keywords.find((k) => k.count > 0)?.count ?? 1;

  return (
    <div className="searchKeywordPanel">
      <div className="searchKeywords">
        {keywords.map((stat) => {
          const isActive = selected.has(stat.keyword);
          const isCustom = stat.count === 0;
          // Scale font 0.78rem → 1.15rem based on relative frequency
          const weight = isCustom ? 0 : stat.count / maxCount;
          const fontSize = 0.78 + weight * 0.37;

          return (
            <span
              key={stat.keyword}
              className={`searchKeywordChip ${isActive ? "searchKeywordChipActive" : ""} ${isCustom ? "searchKeywordChipCustom" : ""}`}
              style={{ fontSize: `${fontSize}rem` }}
            >
              <button
                type="button"
                className="searchKeywordChipLabel"
                onClick={() => onToggle(stat.keyword)}
                title={
                  isCustom
                    ? "Custom keyword — click to toggle"
                    : `Used in ${stat.count} saved job${stat.count === 1 ? "" : "s"} — click to toggle`
                }
              >
                {stat.keyword}
                {!isCustom && (
                  <span className="searchKeywordCount">×{stat.count}</span>
                )}
              </button>
              {isCustom && (
                <button
                  type="button"
                  className="searchKeywordRemove"
                  onClick={() => onRemoveCustom(stat.keyword)}
                  aria-label={`Remove ${stat.keyword}`}
                  title="Remove custom keyword"
                >
                  ×
                </button>
              )}
            </span>
          );
        })}
      </div>

      <div className="searchCustomInput">
        <input
          className="searchCustomInputField"
          value={customKeyword}
          onChange={(e) => onCustomChange(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              onAddCustom();
            }
          }}
          placeholder={en.jobSearch.addKeywordPh}
        />
        <button type="button" className="btn btnGhost" onClick={onAddCustom}>
          {en.jobSearch.addKeyword}
        </button>
      </div>
    </div>
  );
}
