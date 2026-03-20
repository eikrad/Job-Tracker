import { memo } from "react";
import type { Job } from "../../lib/types";
import { googleCalendarEventLink } from "../../lib/calendar/googleSync";
import { googleCalendarCreateEvent } from "../../lib/tauriApi";
import { en } from "../../i18n/en";

type Props = {
  jobs: Job[];
  googleAccessToken: string;
  onApiSynced?: () => void | Promise<void>;
};

export const DeadlinesView = memo(function DeadlinesView({
  jobs,
  googleAccessToken,
  onApiSynced,
}: Props) {
  const withDeadlines = jobs
    .filter((j) => j.deadline)
    .sort((a, b) => (a.deadline ?? "").localeCompare(b.deadline ?? ""));

  async function syncViaApi(job: Job) {
    if (!googleAccessToken.trim()) {
      window.alert(en.deadlines.tokenRequired);
      return;
    }
    try {
      const link = await googleCalendarCreateEvent(googleAccessToken.trim(), job.id);
      window.alert(en.deadlines.eventCreated(link));
      await onApiSynced?.();
    } catch (e) {
      window.alert(String(e));
    }
  }

  return (
    <section className="card">
      <h2>{en.deadlines.title}</h2>
      <p className="muted">
        {en.deadlines.intro} <code>https://www.googleapis.com/auth/calendar.events</code>.
      </p>
      <ul>
        {withDeadlines.map((job) => {
          const link = googleCalendarEventLink(job);
          return (
            <li key={job.id}>
              {job.deadline} — {job.company} ({job.title ?? en.common.untitled}){" "}
              {link && (
                <a href={link} target="_blank" rel="noreferrer">
                  {en.deadlines.templateLink}
                </a>
              )}{" "}
              <button type="button" onClick={() => void syncViaApi(job)}>
                {en.deadlines.createViaApi}
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
});
