import { memo, useMemo, useState } from "react";
import type { Job } from "../../lib/types";
import { WorkspaceEmpty } from "../../components/WorkspaceEmpty";
import { en } from "../../i18n/en";

type Props = { jobs: Job[]; onSelect: (job: Job) => void };
type SortDirection = "desc" | "asc";
type SortKey = "company" | "title" | "status" | "priority" | "deadline" | "interview_date" | "start_date" | "detected_language";

function compareNullableStrings(a: string | null | undefined, b: string | null | undefined) {
  const left = (a ?? "").trim().toLocaleLowerCase();
  const right = (b ?? "").trim().toLocaleLowerCase();
  if (!left && !right) return 0;
  if (!left) return 1;
  if (!right) return -1;
  return left.localeCompare(right);
}

function compareNullableDates(a: string | null | undefined, b: string | null | undefined) {
  if (!a && !b) return 0;
  if (!a) return 1;
  if (!b) return -1;
  return a.localeCompare(b);
}

function sortJobs(jobs: Job[], sortKey: SortKey, sortDirection: SortDirection) {
  const copy = [...jobs];
  copy.sort((a, b) => {
    let result = 0;
    switch (sortKey) {
      case "priority": {
        const aPriority = a.priority ?? -1;
        const bPriority = b.priority ?? -1;
        result = aPriority - bPriority;
        break;
      }
      case "deadline":
      case "interview_date":
      case "start_date":
        result = compareNullableDates(a[sortKey], b[sortKey]);
        break;
      case "company":
      case "title":
      case "status":
      case "detected_language":
        result = compareNullableStrings(a[sortKey], b[sortKey]);
        break;
      default: {
        const _never: never = sortKey;
        return _never;
      }
    }
    if (result === 0) return b.id - a.id;
    return sortDirection === "desc" ? -result : result;
  });
  return copy;
}

function sortIndicator(currentKey: SortKey, key: SortKey, direction: SortDirection) {
  if (currentKey !== key) return "";
  return direction === "desc" ? "↓" : "↑";
}

function handleRowKeyDown(e: React.KeyboardEvent, callback: () => void) {
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    callback();
  }
}

export const JobTable = memo(function JobTable({ jobs, onSelect }: Props) {
  const [sortKey, setSortKey] = useState<SortKey>("priority");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");

  function onSort(nextKey: SortKey) {
    if (nextKey === sortKey) {
      setSortDirection((v) => (v === "desc" ? "asc" : "desc"));
      return;
    }
    setSortKey(nextKey);
    setSortDirection("desc");
  }

  const sortedJobs = useMemo(
    () => sortJobs(jobs, sortKey, sortDirection),
    [jobs, sortDirection, sortKey],
  );

  return (
    <section className="card">
      <h2>{en.jobTable.title}</h2>
      {jobs.length === 0 ? (
        <WorkspaceEmpty
          title={en.empty.tableTitle}
          body={en.empty.tableBody}
          cta={en.empty.tableCta}
        />
      ) : (
        <div className="tableWrap">
          <table>
            <thead>
              <tr>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("company")}
                    aria-label={en.jobTable.sortByCompany}
                  >
                    {en.jobTable.company} {sortIndicator(sortKey, "company", sortDirection)}
                  </button>
                </th>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("title")}
                    aria-label={en.jobTable.sortByTitle}
                  >
                    {en.jobTable.titleCol} {sortIndicator(sortKey, "title", sortDirection)}
                  </button>
                </th>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("status")}
                    aria-label={en.jobTable.sortByStatus}
                  >
                    {en.jobTable.status} {sortIndicator(sortKey, "status", sortDirection)}
                  </button>
                </th>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("priority")}
                    aria-label={en.jobTable.sortByRating}
                  >
                    {en.jobTable.rating} {sortIndicator(sortKey, "priority", sortDirection)}
                  </button>
                </th>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("deadline")}
                    aria-label={en.jobTable.sortByDeadline}
                  >
                    {en.jobTable.deadline} {sortIndicator(sortKey, "deadline", sortDirection)}
                  </button>
                </th>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("interview_date")}
                    aria-label={en.jobTable.sortByInterview}
                  >
                    {en.jobTable.interview} {sortIndicator(sortKey, "interview_date", sortDirection)}
                  </button>
                </th>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("start_date")}
                    aria-label={en.jobTable.sortByStart}
                  >
                    {en.jobTable.start} {sortIndicator(sortKey, "start_date", sortDirection)}
                  </button>
                </th>
                <th>
                  <button
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => onSort("detected_language")}
                    aria-label={en.jobTable.sortByLanguage}
                  >
                    {en.jobTable.language} {sortIndicator(sortKey, "detected_language", sortDirection)}
                  </button>
                </th>
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
                  <td>{job.company}</td>
                  <td>{job.title ?? en.common.dash}</td>
                  <td>{job.status}</td>
                  <td>{job.priority ?? en.common.dash}</td>
                  <td>{job.deadline ?? en.common.dash}</td>
                  <td>{job.interview_date ?? en.common.dash}</td>
                  <td>{job.start_date ?? en.common.dash}</td>
                  <td>{job.detected_language ?? en.common.dash}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
});
