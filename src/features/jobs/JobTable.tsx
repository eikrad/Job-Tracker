import { memo, useEffect, useMemo, useState } from "react";
import type { Job } from "../../lib/types";
import {
  buildJobTableColumns,
  loadVisibleJobTableColumns,
  saveVisibleJobTableColumns,
  toggleVisibleColumn,
  type JobTableColumnId,
} from "../../lib/jobs/jobTableColumns";
import {
  sortJobs,
  type JobSortKey,
  type SortDirection,
} from "../../lib/jobs/sortJobs";
import { WorkspaceEmpty } from "../../components/WorkspaceEmpty";
import { en } from "../../i18n/en";

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

export const JobTable = memo(function JobTable({ jobs, statuses, onSelect }: Props) {
  const [primary, setPrimary] = useState<JobSortKey>("status");
  const [primaryDirection, setPrimaryDirection] = useState<SortDirection>("asc");
  const [secondary, setSecondary] = useState<JobSortKey | "none">("company");
  const [secondaryDirection, setSecondaryDirection] = useState<SortDirection>("asc");
  const [visibleColumns, setVisibleColumns] = useState(loadVisibleJobTableColumns);

  useEffect(() => {
    saveVisibleJobTableColumns(visibleColumns);
  }, [visibleColumns]);

  const activeColumns = useMemo(
    () => JOB_TABLE_COLUMNS.filter((col) => visibleColumns.includes(col.id)),
    [visibleColumns],
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

  const sortedJobs = useMemo(
    () =>
      sortJobs(jobs, {
        primary,
        primaryDirection,
        secondary: secondary === "none" ? null : secondary,
        secondaryDirection,
        statusOrder: statuses,
      }),
    [jobs, primary, primaryDirection, secondary, secondaryDirection, statuses],
  );

  return (
    <section className="card jobTableCard">
      <h2>{en.jobTable.title}</h2>
      {jobs.length === 0 ? (
        <WorkspaceEmpty
          title={en.empty.tableTitle}
          body={en.empty.tableBody}
          cta={en.empty.tableCta}
        />
      ) : (
        <>
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
                        <td
                          key={col.id}
                          className={col.cellClass}
                          title={title}
                        >
                          {content}
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  );
});
