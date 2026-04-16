import dayjs from "dayjs";
import { memo, useMemo } from "react";
import type { Job } from "../../lib/types";
import { WorkspaceEmpty } from "../../components/WorkspaceEmpty";
import { en } from "../../i18n/en";

type Props = { jobs: Job[] };

const DONE_STATUS_NAME = "Done";

function urgencyClass(days: number): string {
  if (days < 0) return "reminderOverdue";
  if (days <= 2) return "reminderUrgent";
  if (days <= 7) return "reminderSoon";
  return "reminderOk";
}

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
    <section className="card dashboardPanel">
      <h2>{en.reminders.title}</h2>
      {reminders.length === 0 ? (
        <WorkspaceEmpty
          title={en.empty.remindersTitle}
          body={en.empty.remindersBody}
          cta={en.empty.remindersCta}
        />
      ) : (
        <div className="dashboardPanelScroll">
          <ul className="listPlain reminderList">
            {reminders.map(({ job, days }) => {
              const followUp =
                days <= 7
                  ? en.reminders.followUp7
                  : days <= 14
                    ? en.reminders.followUp14
                    : null;
              return (
                <li key={job.id} className={urgencyClass(days)}>
                  <span className="reminderBadge">
                    {days < 0 ? en.reminders.overdue(Math.abs(days)) : en.reminders.dueIn(days)}
                  </span>
                  {" "}
                  {job.company} - {job.title ?? en.common.untitled}
                  {followUp ? <span className="reminderFollowUp"> {followUp}</span> : ""}
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </section>
  );
});
