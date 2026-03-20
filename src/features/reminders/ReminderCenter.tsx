import dayjs from "dayjs";
import { memo, useMemo } from "react";
import type { Job } from "../../lib/types";
import { en } from "../../i18n/en";

type Props = { jobs: Job[] };

/** Matches default pipeline; reminders ignore jobs moved to this status by name. */
const DONE_STATUS_NAME = "Done";

export const ReminderCenter = memo(function ReminderCenter({ jobs }: Props) {
  const reminders = useMemo(() => {
    const now = dayjs();
    return jobs
      .filter((j) => !!j.deadline && j.status !== DONE_STATUS_NAME)
      .map((j) => {
        const due = dayjs(j.deadline);
        return { job: j, days: due.diff(now, "day") };
      })
      .filter((v) => v.days <= 14)
      .sort((a, b) => a.days - b.days);
  }, [jobs]);

  return (
    <section className="card">
      <h2>{en.reminders.title}</h2>
      <ul>
        {reminders.map(({ job, days }) => {
          const followUp =
            days <= 7
              ? en.reminders.followUp7
              : days <= 14
                ? en.reminders.followUp14
                : null;
          return (
            <li key={job.id}>
              {job.company} - {job.title ?? en.common.untitled}:{" "}
              {days < 0 ? en.reminders.overdue(Math.abs(days)) : en.reminders.dueIn(days)}
              {followUp ? ` | ${followUp}` : ""}
            </li>
          );
        })}
      </ul>
    </section>
  );
});
