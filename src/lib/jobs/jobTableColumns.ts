import type { ReactNode } from "react";
import type { Job } from "../types";
import type { JobSortKey } from "./sortJobs";
import { formatJobAddedDate } from "./sortJobs";

export type JobTableColumnId = JobSortKey;

export const JOB_TABLE_COLUMN_ORDER: JobTableColumnId[] = [
  "company",
  "title",
  "status",
  "priority",
  "created_at",
  "deadline",
  "interview_date",
  "start_date",
  "detected_language",
];

export const DEFAULT_VISIBLE_JOB_TABLE_COLUMNS: JobTableColumnId[] = [
  "company",
  "title",
  "status",
  "priority",
  "created_at",
  "deadline",
];

const STORAGE_KEY = "jobTableVisibleColumns";

export type JobTableColumnDef = {
  id: JobTableColumnId;
  colClass: string;
  cellClass?: string;
  render: (job: Job, labels: { dash: string }) => ReactNode;
};

export function buildJobTableColumns(): JobTableColumnDef[] {
  return [
    {
      id: "company",
      colClass: "jobTableColCompany",
      cellClass: "jobTableCellTruncate",
      render: (job) => job.company,
    },
    {
      id: "title",
      colClass: "jobTableColTitle",
      cellClass: "jobTableCellTruncate",
      render: (job, { dash }) => job.title ?? dash,
    },
    {
      id: "status",
      colClass: "jobTableColStatus",
      cellClass: "jobTableCellTruncate",
      render: (job) => job.status,
    },
    {
      id: "priority",
      colClass: "jobTableColRating",
      render: (job, { dash }) => job.priority ?? dash,
    },
    {
      id: "created_at",
      colClass: "jobTableColAdded",
      cellClass: "jobTableCellDate",
      render: (job) => formatJobAddedDate(job.created_at),
    },
    {
      id: "deadline",
      colClass: "jobTableColDate",
      cellClass: "jobTableCellDate",
      render: (job, { dash }) => job.deadline ?? dash,
    },
    {
      id: "interview_date",
      colClass: "jobTableColDate",
      cellClass: "jobTableCellDate",
      render: (job, { dash }) => job.interview_date ?? dash,
    },
    {
      id: "start_date",
      colClass: "jobTableColDate",
      cellClass: "jobTableCellDate",
      render: (job, { dash }) => job.start_date ?? dash,
    },
    {
      id: "detected_language",
      colClass: "jobTableColLang",
      cellClass: "jobTableCellTruncate",
      render: (job, { dash }) => job.detected_language ?? dash,
    },
  ];
}

export function normalizeVisibleColumns(raw: unknown): JobTableColumnId[] {
  if (!Array.isArray(raw)) return [...DEFAULT_VISIBLE_JOB_TABLE_COLUMNS];
  const allowed = new Set(JOB_TABLE_COLUMN_ORDER);
  const picked = raw.filter((id): id is JobTableColumnId => typeof id === "string" && allowed.has(id as JobTableColumnId));
  const unique = JOB_TABLE_COLUMN_ORDER.filter((id) => picked.includes(id));
  return unique.length > 0 ? unique : [...DEFAULT_VISIBLE_JOB_TABLE_COLUMNS];
}

function storageAvailable(): boolean {
  try {
    return typeof localStorage !== "undefined" && typeof localStorage.setItem === "function";
  } catch {
    return false;
  }
}

export function loadVisibleJobTableColumns(): JobTableColumnId[] {
  if (!storageAvailable()) return [...DEFAULT_VISIBLE_JOB_TABLE_COLUMNS];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [...DEFAULT_VISIBLE_JOB_TABLE_COLUMNS];
    return normalizeVisibleColumns(JSON.parse(raw));
  } catch {
    return [...DEFAULT_VISIBLE_JOB_TABLE_COLUMNS];
  }
}

export function saveVisibleJobTableColumns(columns: JobTableColumnId[]): void {
  if (!storageAvailable()) return;
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(columns));
  } catch {
    /* ignore quota / private mode */
  }
}

export function toggleVisibleColumn(
  current: JobTableColumnId[],
  columnId: JobTableColumnId,
): JobTableColumnId[] {
  if (current.includes(columnId)) {
    if (current.length <= 1) return current;
    return current.filter((id) => id !== columnId);
  }
  return JOB_TABLE_COLUMN_ORDER.filter((id) => current.includes(id) || id === columnId);
}
