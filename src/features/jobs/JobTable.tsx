import { memo, useEffect, useMemo, useState } from "react";
import type { Job } from "../../lib/types";
import {
  DEFAULT_VISIBLE_JOB_TABLE_COLUMNS,
  buildJobTableColumns,
  loadVisibleJobTableColumns,
  saveVisibleJobTableColumns,
  toggleVisibleColumn,
  type JobTableColumnId,
} from "../../lib/jobs/jobTableColumns";
import { filterJobsByHiddenStatuses } from "../../lib/jobs/filterJobs";
import {
  loadHiddenJobStatuses,
  saveHiddenJobStatuses,
  toggleHiddenJobStatus,
} from "../../lib/jobs/hiddenJobStatuses";
import {
  sortJobs,
  type JobSortKey,
  type SortDirection,
} from "../../lib/jobs/sortJobs";
import { WorkspaceEmpty } from "../../components/WorkspaceEmpty";
import { en } from "../../i18n/en";
import { ChevronDown, ExternalLink, SlidersHorizontal } from "lucide-react";
import { openUrlInBrowser } from "../../lib/tauriApi";
import { ListingStatusDot } from "./ListingStatusDot";

type Props = {
  jobs: Job[];
  statuses: string[];
  onSelect: (job: Job) => void;
};

const COLUMN_LABELS: Record<JobTableColumnId, string> = {
  company: en.jobTable.company,
  title: en.jobTable.titleCol,
  status: en.jobTable.status,
  priority: en.jobTable.rating,
  created_at: en.jobTable.added,
  deadline: en.jobTable.deadline,
  interview_date: en.jobTable.interview,
  start_date: en.jobTable.start,
  detected_language: en.jobTable.language,
};

const SORT_ARIA: Record<JobTableColumnId, string> = {
  company: en.jobTable.sortByCompany,
  title: en.jobTable.sortByTitle,
  status: en.jobTable.sortByStatus,
  priority: en.jobTable.sortByRating,
  created_at: en.jobTable.sortByAdded,
  deadline: en.jobTable.sortByDeadline,
  interview_date: en.jobTable.sortByInterview,
  start_date: en.jobTable.sortByStart,
  detected_language: en.jobTable.sortByLanguage,
};

const JOB_TABLE_COLUMNS = buildJobTableColumns();
const SORT_OPTIONS = JOB_TABLE_COLUMNS.map((col) => ({
  value: col.id,
  label: COLUMN_LABELS[col.id],
}));

function sortIndicator(active: boolean, direction: SortDirection) {
  if (!active) return "";
  return direction === "desc" ? " ↓" : " ↑";
}

function handleRowKeyDown(e: React.KeyboardEvent, callback: () => void) {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    callback();
  }
}

function columnsDifferFromDefault(visible: JobTableColumnId[]): boolean {
  if (visible.length !== DEFAULT_VISIBLE_JOB_TABLE_COLUMNS.length) return true;
  return DEFAULT_VISIBLE_JOB_TABLE_COLUMNS.some((id, i) => visible[i] !== id);
}

export const JobTable = memo(function JobTable({ jobs, statuses, onSelect }: Props) {
  const [primary, setPrimary] = useState<JobSortKey>("status");
  const [primaryDirection, setPrimaryDirection] = useState<SortDirection>("asc");
  const [secondary, setSecondary] = useState<JobSortKey | "none">("company");
  const [secondaryDirection, setSecondaryDirection] = useState<SortDirection>("asc");
  const [visibleColumns, setVisibleColumns] = useState(loadVisibleJobTableColumns);
  const [hiddenStatuses, setHiddenStatuses] = useState(() => loadHiddenJobStatuses(statuses));
  const [customizeOpen, setCustomizeOpen] = useState(false);

  useEffect(() => {
    saveVisibleJobTableColumns(visibleColumns);
  }, [visibleColumns]);

  useEffect(() => {
    setHiddenStatuses((current) => current.filter((name) => statuses.includes(name)));
  }, [statuses]);

  useEffect(() => {
    saveHiddenJobStatuses(hiddenStatuses);
  }, [hiddenStatuses]);

  const activeColumns = useMemo(
    () => JOB_TABLE_COLUMNS.filter((col) => visibleColumns.includes(col.id)),
    [visibleColumns],
  );

  const customizeActive = useMemo(
    () => columnsDifferFromDefault(visibleColumns) || hiddenStatuses.length > 0,
    [visibleColumns, hiddenStatuses],
  );

  function onHeaderSort(nextKey: JobSortKey) {
    if (nextKey === primary) {
      setPrimaryDirection((v) => (v === "desc" ? "asc" : "desc"));
      return;
    }
    setPrimary(nextKey);
    setPrimaryDirection("desc");
  }

  function onToggleColumn(columnId: JobTableColumnId) {
    setVisibleColumns((current) => toggleVisibleColumn(current, columnId));
  }

  function onToggleHiddenStatus(status: string) {
    setHiddenStatuses((current) => toggleHiddenJobStatus(current, status, statuses));
  }

  const visibleJobs = useMemo(
    () => filterJobsByHiddenStatuses(jobs, hiddenStatuses),
    [jobs, hiddenStatuses],
  );

  const sortedJobs = useMemo(
    () =>
      sortJobs(visibleJobs, {
        primary,
        primaryDirection,
        secondary: secondary === "none" ? null : secondary,
        secondaryDirection,
        statusOrder: statuses,
      }),
    [visibleJobs, primary, primaryDirection, secondary, secondaryDirection, statuses],
  );

  return (
    <section className="card jobTableCard">
      <div className="jobTableHeader">
        <h2>{en.jobTable.title}</h2>
        {jobs.length > 0 && (
          <button
            type="button"
            className={`btn btnGhost btnSm jobTableCustomizeToggle ${
              customizeOpen ? "jobTableCustomizeToggleOpen" : ""
            }`}
            aria-expanded={customizeOpen}
            aria-controls="job-table-customize-panel"
            aria-label={en.jobTable.customizeAria}
            title={customizeActive ? en.jobTable.customizeActiveHint : undefined}
            onClick={() => setCustomizeOpen((open) => !open)}
          >
            <SlidersHorizontal size={14} aria-hidden />
            {customizeOpen ? en.jobTable.customizeHide : en.jobTable.customize}
            {customizeActive && !customizeOpen ? (
              <span className="jobTableCustomizeBadge" aria-hidden />
            ) : null}
            <ChevronDown
              size={14}
              aria-hidden
              className={`jobTableCustomizeChevron ${customizeOpen ? "isOpen" : ""}`}
            />
          </button>
        )}
      </div>
      {jobs.length === 0 ? (
        <WorkspaceEmpty
          title={en.empty.tableTitle}
          body={en.empty.tableBody}
          cta={en.empty.tableCta}
        />
      ) : (
        <>
          {customizeOpen && (
            <div
              id="job-table-customize-panel"
              className="jobTableCustomizePanel"
              role="region"
              aria-label={en.jobTable.customizeAria}
            >
              <fieldset className="jobTableColumnPicker">
                <legend>{en.jobTable.columnsLegend}</legend>
                <div className="jobTableColumnChecks">
                  {JOB_TABLE_COLUMNS.map((col) => {
                    const checked = visibleColumns.includes(col.id);
                    const isLastVisible = checked && visibleColumns.length === 1;
                    return (
                      <label key={col.id} className="jobTableColumnCheck">
                        <input
                          type="checkbox"
                          checked={checked}
                          disabled={isLastVisible}
                          onChange={() => onToggleColumn(col.id)}
                        />
                        <span>{COLUMN_LABELS[col.id]}</span>
                      </label>
                    );
                  })}
                </div>
              </fieldset>
              <fieldset className="jobTableColumnPicker">
                <legend>{en.jobTable.hideStatusesLegend}</legend>
                <div className="jobTableColumnChecks">
                  {statuses.map((status) => {
                    const checked = hiddenStatuses.includes(status);
                    return (
                      <label key={status} className="jobTableColumnCheck">
                        <input
                          type="checkbox"
                          checked={checked}
                          onChange={() => onToggleHiddenStatus(status)}
                        />
                        <span>{status}</span>
                      </label>
                    );
                  })}
                </div>
              </fieldset>
            </div>
          )}
          <div className="jobTableSortBar" role="group" aria-label={en.jobTable.sortControls}>
            <div className="jobTableSortField">
              <span className="jobTableSortLabel" id="job-table-sort-primary-label">
                {en.jobTable.sortPrimary}
              </span>
              <select
                value={primary}
                aria-labelledby="job-table-sort-primary-label"
                onChange={(e) => setPrimary(e.target.value as JobSortKey)}
              >
                {SORT_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="btn btnGhost btnSm jobTableSortDirBtn"
                onClick={() => setPrimaryDirection((v) => (v === "desc" ? "asc" : "desc"))}
                aria-label={en.jobTable.togglePrimaryDirection}
              >
                {primaryDirection === "desc" ? "↓" : "↑"}
              </button>
            </div>
            <div className="jobTableSortField">
              <span className="jobTableSortLabel" id="job-table-sort-secondary-label">
                {en.jobTable.sortSecondary}
              </span>
              <select
                value={secondary}
                aria-labelledby="job-table-sort-secondary-label"
                onChange={(e) => setSecondary(e.target.value as JobSortKey | "none")}
              >
                <option value="none">{en.jobTable.sortNone}</option>
                {SORT_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="btn btnGhost btnSm jobTableSortDirBtn"
                disabled={secondary === "none"}
                onClick={() => setSecondaryDirection((v) => (v === "desc" ? "asc" : "desc"))}
                aria-label={en.jobTable.toggleSecondaryDirection}
              >
                {secondaryDirection === "desc" ? "↓" : "↑"}
              </button>
            </div>
          </div>
          {sortedJobs.length === 0 ? (
            <p className="muted jobTableFilteredEmpty">{en.empty.tableFilteredBody}</p>
          ) : (
            <div className="tableWrap jobTableWrap">
              <table className="jobTable">
                <colgroup>
                  {activeColumns.map((col) => (
                    <col key={col.id} className={col.colClass} />
                  ))}
                </colgroup>
                <thead>
                  <tr>
                    {activeColumns.map((col) => (
                      <th key={col.id}>
                        <button
                          type="button"
                          className="btn btnGhost btnSm jobTableSortBtn"
                          onClick={() => onHeaderSort(col.id)}
                          aria-label={SORT_ARIA[col.id]}
                        >
                          {COLUMN_LABELS[col.id]}
                          {sortIndicator(primary === col.id, primaryDirection)}
                          {secondary === col.id ? (
                            <span
                              className="jobTableSecondaryMark"
                              title={en.jobTable.secondarySortMark}
                            >
                              2
                            </span>
                          ) : null}
                        </button>
                      </th>
                    ))}
                    <th className="jobTableLinkCell" aria-label="Link" />
                  </tr>
                </thead>
                <tbody>
                  {sortedJobs.map((job) => (
                    <tr
                      key={job.id}
                      tabIndex={0}
                      role="button"
                      onClick={() => onSelect(job)}
                      onKeyDown={(e) => handleRowKeyDown(e, () => onSelect(job))}
                      aria-label={`View details for ${job.company} - ${job.title ?? "Untitled"}`}
                    >
                      {activeColumns.map((col) => {
                        const content = col.render(job, { dash: en.common.dash });
                        const title =
                          col.id === "company"
                            ? job.company
                            : col.id === "title"
                              ? (job.title ?? undefined)
                              : col.id === "status"
                                ? job.status
                                : undefined;
                        return (
                          <td key={col.id} className={col.cellClass} title={title}>
                            {content}
                          </td>
                        );
                      })}
                      <td className="jobTableLinkCell" onClick={(e) => e.stopPropagation()}>
                        {job.url && (
                          <div className="jobTableLinkInner">
                            <ListingStatusDot status={job.listing_status} />
                            <a
                              href={job.url}
                              target="_blank"
                              rel="noopener noreferrer"
                              title={job.url}
                              onClick={(e) => {
                                e.preventDefault();
                                void openUrlInBrowser(job.url!.trim()).catch(console.error);
                              }}
                            >
                              <ExternalLink size={13} />
                            </a>
                          </div>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      )}
    </section>
  );
});
