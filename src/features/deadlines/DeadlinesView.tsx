import type { Job } from "../../lib/types";
import { googleCalendarEventLink } from "../../lib/calendar/googleSync";
import { googleCalendarCreateEvent } from "../../lib/tauriApi";

type Props = {
  jobs: Job[];
  googleAccessToken: string;
  onApiSynced?: () => void;
};

export function DeadlinesView({ jobs, googleAccessToken, onApiSynced }: Props) {
  const withDeadlines = jobs
    .filter((j) => j.deadline)
    .sort((a, b) => (a.deadline ?? "").localeCompare(b.deadline ?? ""));

  async function syncViaApi(job: Job) {
    if (!googleAccessToken.trim()) {
      window.alert("Add a Google OAuth access token (Calendar scope) in settings above.");
      return;
    }
    try {
      const link = await googleCalendarCreateEvent(googleAccessToken.trim(), job.id);
      window.alert(`Event created. Open: ${link}`);
      onApiSynced?.();
    } catch (e) {
      window.alert(String(e));
    }
  }

  return (
    <section className="card">
      <h2>Calendar View</h2>
      <p className="muted">
        Quick add: template link (no API). API: creates an event in your primary calendar using an OAuth access token
        with <code>https://www.googleapis.com/auth/calendar.events</code>.
      </p>
      <ul>
        {withDeadlines.map((job) => {
          const link = googleCalendarEventLink(job);
          return (
            <li key={job.id}>
              {job.deadline} — {job.company} ({job.title ?? "Untitled"}){" "}
              {link && (
                <a href={link} target="_blank" rel="noreferrer">
                  Template link
                </a>
              )}{" "}
              <button type="button" onClick={() => void syncViaApi(job)}>
                Create via API
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
