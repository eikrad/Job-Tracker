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
        {reminders.map(({ job, days }) => (
          <li key={job.id}>
            {job.company} - {job.title ?? "Untitled"}: {days < 0 ? `Overdue by ${Math.abs(days)}d` : `Due in ${days}d`}
          </li>
        ))}
      </ul>
    </section>
  );
}
