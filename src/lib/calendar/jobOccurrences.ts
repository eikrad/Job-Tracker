import dayjs from "dayjs";
import type { Job } from "../types";

export type CalendarDateKind = "apply" | "interview" | "start";

export type JobOccurrence = {
  jobId: number;
  company: string;
  title: string | null;
  date: string;
  kind: CalendarDateKind;
};

/** Flatten job dates into sortable occurrences (YYYY-MM-DD). */
export function jobsToOccurrences(jobs: Job[]): JobOccurrence[] {
  const out: JobOccurrence[] = [];
  for (const j of jobs) {
    if (j.deadline?.trim()) {
      out.push({
        jobId: j.id,
        company: j.company,
        title: j.title ?? null,
        date: j.deadline.trim(),
        kind: "apply",
      });
    }
    if (j.interview_date?.trim()) {
      out.push({
        jobId: j.id,
        company: j.company,
        title: j.title ?? null,
        date: j.interview_date.trim(),
        kind: "interview",
      });
    }
    if (j.start_date?.trim()) {
      out.push({
        jobId: j.id,
        company: j.company,
        title: j.title ?? null,
        date: j.start_date.trim(),
        kind: "start",
      });
    }
  }
  out.sort((a, b) => {
    const c = a.date.localeCompare(b.date);
    if (c !== 0) return c;
    return a.company.localeCompare(b.company);
  });
  return out;
}

/** Group occurrences that fall inside the given month (dayjs month is 0–11). */
export function occurrencesByDateKey(
  occurrences: JobOccurrence[],
  year: number,
  monthIndex: number,
): Map<string, JobOccurrence[]> {
  const start = dayjs(new Date(year, monthIndex, 1)).startOf("month");
  const end = start.endOf("month");
  const map = new Map<string, JobOccurrence[]>();
  for (const o of occurrences) {
    const d = dayjs(o.date);
    if (!d.isValid()) continue;
    if (d.isBefore(start, "day") || d.isAfter(end, "day")) continue;
    const key = o.date;
    const list = map.get(key) ?? [];
    list.push(o);
    map.set(key, list);
  }
  return map;
}

/** Weekday headers starting Monday (0 = Mon … 6 = Sun). */
export function weekdayLabelsMonFirst(): string[] {
  return ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
}

/** Build 6×7 grid cells for month view: null = empty, number = day of month. */
export function monthGridDays(year: number, monthIndex: number): (number | null)[][] {
  const first = dayjs(new Date(year, monthIndex, 1));
  const daysInMonth = first.daysInMonth();
  // Monday-first: convert JS Sunday=0 to Monday=0 index
  let startDow = first.day() - 1;
  if (startDow < 0) startDow = 6;
  const rows: (number | null)[][] = [];
  let day = 1 - startDow;
  for (let r = 0; r < 6; r++) {
    const row: (number | null)[] = [];
    for (let c = 0; c < 7; c++) {
      if (day < 1 || day > daysInMonth) {
        row.push(null);
      } else {
        row.push(day);
      }
      day += 1;
    }
    rows.push(row);
  }
  return rows;
}

export function dateKeyFromParts(year: number, monthIndex: number, dayOfMonth: number): string {
  const m = monthIndex + 1;
  const mm = m < 10 ? `0${m}` : `${m}`;
  const dd = dayOfMonth < 10 ? `0${dayOfMonth}` : `${dayOfMonth}`;
  return `${year}-${mm}-${dd}`;
}
