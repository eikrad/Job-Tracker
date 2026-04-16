import { en } from "../i18n/en";
import { useJobSearch, INDEED_REGIONS } from "../features/jobSearch/useJobSearch";
import { KeywordPanel } from "../features/jobSearch/KeywordPanel";
import { PlatformResultsList } from "../features/jobSearch/PlatformResultsList";
import { SearchResultCard } from "../features/jobSearch/SearchResultCard";
import type { Platform } from "../features/jobSearch/useJobSearch";

const PLATFORMS: { id: Platform; label: string }[] = [
  { id: "jobindex", label: "Jobindex.dk" },
  { id: "indeed", label: "Indeed" },
  { id: "linkedin", label: "LinkedIn" },
  { id: "thehub", label: "The Hub" },
];

export function JobSearchPage() {
  const {
    allKeywords,
    selectedKeywords,
    customKeyword,
    setCustomKeyword,
    toggleKeyword,
    addCustomKeyword,
    removeCustomKeyword,
    locationSuggestions,
    location,
    setLocation,
    indeedRegion,
    setIndeedRegion,
    activePlatforms,
    togglePlatform,
    results,
    globalTop5,
    viewMode,
    setViewMode,
    fallbackHints,
    loading,
    errors,
    hasSearched,
    search,
    openInBrowser,
  } = useJobSearch();

  const selectedCount = selectedKeywords.size;
  const isSearching = loading.jobindex || loading.indeed;

  return (
    <div className="jobSearchPage">
      <div className="jobSearchTop">
        <h1 className="jobSearchTitle">{en.jobSearch.title}</h1>
        <p className="muted jobSearchSubtitle">{en.jobSearch.subtitle}</p>
      </div>

      {/* ── Controls card ── */}
      <div className="card jobSearchCard">

        {/* Keywords */}
        <section className="jobSearchSection">
          <h2 className="jobSearchSectionTitle">{en.jobSearch.keywordsLabel}</h2>
          <p className="muted jobSearchHint">{en.jobSearch.keywordsHint}</p>
          {allKeywords.length === 0 && (
            <p className="muted">{en.jobSearch.noKeywords}</p>
          )}
          <KeywordPanel
            keywords={allKeywords}
            selected={selectedKeywords}
            onToggle={toggleKeyword}
            onRemoveCustom={removeCustomKeyword}
            customKeyword={customKeyword}
            onCustomChange={setCustomKeyword}
            onAddCustom={addCustomKeyword}
          />
        </section>

        <div className="jobSearchDivider" />

        {/* Location */}
        <section className="jobSearchSection">
          <h2 className="jobSearchSectionTitle">{en.jobSearch.locationLabel}</h2>
          <div className="searchLocationRow">
            <input
              list="locationSuggestions"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              placeholder={en.jobSearch.locationPh}
              className="searchLocationInput"
            />
            <datalist id="locationSuggestions">
              {locationSuggestions.map((c) => (
                <option key={c} value={c} />
              ))}
            </datalist>
            {location && (
              <button
                type="button"
                className="btn btnGhost btnSm"
                onClick={() => setLocation("")}
              >
                {en.jobSearch.clearLocation}
              </button>
            )}
          </div>
        </section>

        <div className="jobSearchDivider" />

        {/* Platforms */}
        <section className="jobSearchSection">
          <h2 className="jobSearchSectionTitle">{en.jobSearch.platformsLabel}</h2>
          <div className="searchPlatforms">
            {PLATFORMS.map(({ id, label }) => (
              <label key={id} className="searchPlatformToggle">
                <input
                  type="checkbox"
                  checked={activePlatforms.has(id)}
                  onChange={() => togglePlatform(id)}
                />
                <span>{label}</span>
              </label>
            ))}
          </div>

          {activePlatforms.has("indeed") && (
            <div className="searchIndeedRegion">
              <label htmlFor="indeedRegion" className="searchIndeedRegionLabel">
                {en.jobSearch.indeedRegion}
              </label>
              <select
                id="indeedRegion"
                value={indeedRegion}
                onChange={(e) => setIndeedRegion(e.target.value)}
              >
                {INDEED_REGIONS.map((r) => (
                  <option key={r.code} value={r.code}>
                    {r.label}
                  </option>
                ))}
              </select>
            </div>
          )}
        </section>

        <div className="jobSearchDivider" />

        {/* Search button */}
        <div className="jobSearchActions">
          <button
            type="button"
            className="btn btnPrimary"
            onClick={search}
            disabled={selectedCount === 0 || isSearching}
          >
            {isSearching ? en.jobSearch.loading : en.jobSearch.search}
          </button>
          {selectedCount > 0 && (
            <span className="muted">
              {en.jobSearch.searchingWith(selectedCount)}
            </span>
          )}
        </div>
      </div>

      {/* ── Results ── */}
      {hasSearched && (
        <div className="searchResultSections">
          <section className="platformSection">
            <div className="platformHeader">
              <h2 className="platformTitle">{en.jobSearch.rankModeTitle}</h2>
              <div className="searchPlatforms">
                <label className="searchPlatformToggle">
                  <input
                    type="radio"
                    name="rankMode"
                    checked={viewMode === "global"}
                    onChange={() => setViewMode("global")}
                  />
                  <span>{en.jobSearch.rankModeGlobal}</span>
                </label>
                <label className="searchPlatformToggle">
                  <input
                    type="radio"
                    name="rankMode"
                    checked={viewMode === "perPlatform"}
                    onChange={() => setViewMode("perPlatform")}
                  />
                  <span>{en.jobSearch.rankModePerPlatform}</span>
                </label>
              </div>
            </div>
            <div className="searchScoreLegend" aria-label={en.jobSearch.scoreLegendTitle}>
              <span className="searchScoreBadge searchScoreToneHigh">{en.jobSearch.scoreLegendHigh}</span>
              <span className="searchScoreBadge searchScoreToneMedium">{en.jobSearch.scoreLegendMedium}</span>
              <span className="searchScoreBadge searchScoreToneLow">{en.jobSearch.scoreLegendLow}</span>
            </div>
          </section>

          {viewMode === "global" && (
            <section className="platformSection">
              <div className="platformHeader">
                <h2 className="platformTitle">
                  {en.jobSearch.globalTopFive}
                  {globalTop5.length > 0 && (
                    <span className="platformCount">{en.jobSearch.resultsCount(globalTop5.length)}</span>
                  )}
                </h2>
              </div>
              {globalTop5.length === 0 ? (
                <p className="muted">{en.jobSearch.noResults}</p>
              ) : (
                <div className="searchResultList">
                  {globalTop5.map((result) => (
                    <SearchResultCard key={`${result.platform}|${result.url}|global`} result={result} />
                  ))}
                </div>
              )}
            </section>
          )}

          {PLATFORMS.map(({ id }) => (
            <PlatformResultsList
              key={id}
              platform={id}
              isActive={activePlatforms.has(id)}
              hidden={viewMode === "global"}
              results={results[id]}
              loading={loading[id]}
              error={errors[id]}
              fallbackHint={fallbackHints[id]}
              onOpenInBrowser={openInBrowser}
            />
          ))}
        </div>
      )}
    </div>
  );
}
