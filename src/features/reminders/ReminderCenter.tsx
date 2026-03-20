import dayjs from "dayjs";
import type { Job } from "../../lib/types";

type Props = { jobs: Job[] };

export function ReminderCenter({ jobs }: Props) {
  const now = dayjs();
  const reminders = jobs
    .filter((j) => !!j.deadline && j.status !== "Done")
    .map((j) => {
      const due = dayjs(j.deadline);
      return { job: j, days: due.diff(now, "day") };
    })
    .filter((v) => v.days <= 14)
    .sort((a, b) => a.days - b.days);

  return (
    <section className="card">
      <h2>Reminders</h2>
      <ul>
        {reminders.map(({ job, days }) => {
          const followUp =
            days <= 7 ? "Suggested follow-up: ~7 days" : days <= 14 ? "Suggested follow-up: ~14 days" : null;
          return (
            <li key={job.id}>
              {job.company} - {job.title ?? "Untitled"}:{" "}
              {days < 0 ? `Overdue by ${Math.abs(days)}d` : `Due in ${days}d`}
              {followUp ? ` | ${followUp}` : ""}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
