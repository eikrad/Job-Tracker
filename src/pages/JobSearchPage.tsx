import { en } from "../i18n/en";
import { useJobSearch, INDEED_REGIONS } from "../features/jobSearch/useJobSearch";
import { KeywordPanel } from "../features/jobSearch/KeywordPanel";
import { PlatformResultsList } from "../features/jobSearch/PlatformResultsList";
import type { Platform } from "../features/jobSearch/useJobSearch";

const PLATFORMS: { id: Platform; label: string }[] = [
  { id: "jobindex", label: "Jobindex.dk" },
  { id: "indeed", label: "Indeed" },
  { id: "linkedin", label: "LinkedIn" },
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
    loading,
    errors,
    hasSearched,
    linkedinOpened,
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
          {allKeywords.length === 0 ? (
            <p className="muted">{en.jobSearch.noKeywords}</p>
          ) : (
            <KeywordPanel
              keywords={allKeywords}
              selected={selectedKeywords}
              onToggle={toggleKeyword}
              onRemoveCustom={removeCustomKeyword}
              customKeyword={customKeyword}
              onCustomChange={setCustomKeyword}
              onAddCustom={addCustomKeyword}
            />
          )}
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
                {id === "linkedin" && (
                  <span className="muted searchPlatformNote">
                    {" "}({en.jobSearch.browserOnly})
                  </span>
                )}
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
          {PLATFORMS.map(({ id }) => (
            <PlatformResultsList
              key={id}
              platform={id}
              isActive={activePlatforms.has(id)}
              results={results[id]}
              loading={loading[id]}
              error={errors[id]}
              linkedinOpened={linkedinOpened}
              onOpenInBrowser={openInBrowser}
            />
          ))}
        </div>
      )}
    </div>
  );
}
