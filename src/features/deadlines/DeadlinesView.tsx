import type { Job } from "../../lib/types";
import { googleCalendarEventLink } from "../../lib/calendar/googleSync";

type Props = { jobs: Job[] };

export function DeadlinesView({ jobs }: Props) {
  const withDeadlines = jobs
    .filter((j) => j.deadline)
    .sort((a, b) => (a.deadline ?? "").localeCompare(b.deadline ?? ""));

  return (
    <section className="card">
      <h2>Calendar View</h2>
      <ul>
        {withDeadlines.map((job) => {
          const link = googleCalendarEventLink(job);
          return (
            <li key={job.id}>
              {job.deadline} - {job.company} ({job.title ?? "Untitled"}){" "}
              {link && <a href={link} target="_blank" rel="noreferrer">Sync to Google</a>}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
