import dayjs from "dayjs";
import { memo, useMemo, useState } from "react";
import type { Job } from "../../lib/types";
import { en } from "../../i18n/en";
import {
  dateKeyFromParts,
  jobsToOccurrences,
  monthGridDays,
  occurrencesByDateKey,
  type JobOccurrence,
  weekdayLabelsMonFirst,
} from "../../lib/calendar/jobOccurrences";
import { googleCalendarTemplateUrl } from "../../lib/calendar/googleSync";
import type { GoogleCalendarDateKind } from "../../lib/tauriApi";

type Props = {
  jobs: Job[];
  selected?: Job;
  onSelectJob: (job: Job) => void;
  googleOauthConnected: boolean;
  hasManualGoogleToken: boolean;
  onCreateInGoogle: (jobId: number, dateKind: GoogleCalendarDateKind) => Promise<string>;
  onOpenSettings: () => void;
};

function kindLabel(kind: JobOccurrence["kind"]): string {
  switch (kind) {
    case "apply":
      return en.deadlines.dateLineApply;
    case "interview":
      return en.deadlines.dateLineInterview;
    case "start":
      return en.deadlines.dateLineStart;
    default:
      return kind;
  }
}

function summaryForTemplate(o: JobOccurrence): string {
  const base = o.company;
  switch (o.kind) {
    case "apply":
      return `${en.deadlines.templateSummaryApply}: ${base}`;
    case "interview":
      return `${en.deadlines.templateSummaryInterview}: ${base}`;
    case "start":
      return `${en.deadlines.templateSummaryStart}: ${base}`;
    default:
      return base;
  }
}

export const JobCalendarMonth = memo(function JobCalendarMonth({
  jobs,
  selected,
  onSelectJob,
  googleOauthConnected,
  hasManualGoogleToken,
  onCreateInGoogle,
  onOpenSettings,
}: Props) {
  const [anchor, setAnchor] = useState(() => dayjs());
  const year = anchor.year();
  const monthIndex = anchor.month();

  const occurrences = useMemo(() => jobsToOccurrences(jobs), [jobs]);
  const byDay = useMemo(
    () => occurrencesByDateKey(occurrences, year, monthIndex),
    [occurrences, year, monthIndex],
  );
  const grid = useMemo(() => monthGridDays(year, monthIndex), [year, monthIndex]);
  const weekdays = weekdayLabelsMonFirst();

  const jobById = useMemo(() => new Map(jobs.map((j) => [j.id, j])), [jobs]);

  const canUseGoogleApi = googleOauthConnected || hasManualGoogleToken;

  async function handleCreate(o: JobOccurrence) {
    if (!canUseGoogleApi) {
      window.alert(en.deadlines.connectOrTokenHint);
      return;
    }
    try {
      const link = await onCreateInGoogle(o.jobId, o.kind);
      window.alert(en.deadlines.eventCreated(link));
    } catch (e) {
      window.alert(String(e));
    }
  }

  return (
    <div className="jobCalendarMonth">
      <div className="jobCalendarToolbar row">
        <button type="button" className="btn btnGhost btnSm" onClick={() => setAnchor((a) => a.subtract(1, "month"))}>
          {en.deadlines.monthPrev}
        </button>
        <h2 className="jobCalendarMonthTitle">{anchor.format("MMMM YYYY")}</h2>
        <button type="button" className="btn btnGhost btnSm" onClick={() => setAnchor((a) => a.add(1, "month"))}>
          {en.deadlines.monthNext}
        </button>
        <button type="button" className="btn btnGhost btnSm" onClick={() => setAnchor(dayjs())}>
          {en.deadlines.monthToday}
        </button>
      </div>
      <p className="muted jobCalendarHint">{en.deadlines.monthIntro}</p>
      {!canUseGoogleApi && (
        <p className="muted jobCalendarConnectHint">
          {en.deadlines.notConnectedHint}{" "}
          <button type="button" className="btn btnLinkLike" onClick={onOpenSettings}>
            {en.deadlines.openSettingsLink}
          </button>
        </p>
      )}
      <div className="jobCalendarGrid" role="grid" aria-label={en.deadlines.title}>
        <div className="jobCalendarWeekdays" role="row">
          {weekdays.map((w) => (
            <div key={w} className="jobCalendarWeekday" role="columnheader">
              {w}
            </div>
          ))}
        </div>
        {grid.map((row, ri) => (
          <div key={ri} className="jobCalendarRow" role="row">
            {row.map((cell, ci) => {
              if (cell === null) {
                return <div key={ci} className="jobCalendarCell jobCalendarCell--empty" role="gridcell" />;
              }
              const dateKey = dateKeyFromParts(year, monthIndex, cell);
              const list = byDay.get(dateKey) ?? [];
              const isToday = dayjs().isSame(dayjs(dateKey), "day");
              return (
                <div
                  key={ci}
                  className={`jobCalendarCell ${isToday ? "jobCalendarCell--today" : ""}`}
                  role="gridcell"
                >
                  <div className="jobCalendarDayNum">{cell}</div>
                  <ul className="jobCalendarOccurrences">
                    {list.map((o, idx) => {
                      const job = jobById.get(o.jobId);
                      const isSel = selected?.id === o.jobId;
                      return (
                        <li key={`${o.jobId}-${o.kind}-${idx}`} className="jobCalendarOccItem">
                          <button
                            type="button"
                            className={`jobCalendarOccMain ${isSel ? "jobCalendarOccMain--selected" : ""}`}
                            onClick={() => job && onSelectJob(job)}
                          >
                            <span className="jobCalendarOccKind">{kindLabel(o.kind)}</span>
                            <span className="jobCalendarOccCo">{o.company}</span>
                          </button>
                          <div className="row jobCalendarOccActions">
                            <a
                              className="btn btnSm btnGhost"
                              href={googleCalendarTemplateUrl({
                                date: o.date,
                                summaryLine: summaryForTemplate(o),
                                details: o.title,
                              })}
                              target="_blank"
                              rel="noreferrer"
                            >
                              {en.deadlines.templateLink}
                            </a>
                            <button type="button" className="btn btnSm btnGhost" onClick={() => void handleCreate(o)}>
                              {en.deadlines.createViaApi}
                            </button>
                          </div>
                        </li>
                      );
                    })}
                  </ul>
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
});
