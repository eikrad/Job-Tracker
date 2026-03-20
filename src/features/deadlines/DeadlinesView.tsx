import { memo } from "react";
import type { Job } from "../../lib/types";
import { WorkspaceEmpty } from "../../components/WorkspaceEmpty";
import { en } from "../../i18n/en";
import { JobCalendarMonth } from "./JobCalendarMonth";
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

export const DeadlinesView = memo(function DeadlinesView({
  jobs,
  selected,
  onSelectJob,
  googleOauthConnected,
  hasManualGoogleToken,
  onCreateInGoogle,
  onOpenSettings,
}: Props) {
  const hasAnyDate = jobs.some((j) => j.deadline || j.interview_date || j.start_date);

  if (!hasAnyDate) {
    return (
      <section className="card">
        <h2>{en.deadlines.title}</h2>
        <p className="muted">{en.deadlines.pageIntro}</p>
        <WorkspaceEmpty
          title={en.empty.calendarTitle}
          body={en.empty.calendarBody}
          cta={en.empty.calendarCta}
        />
      </section>
    );
  }

  return (
    <section className="card">
      <h2>{en.deadlines.title}</h2>
      <p className="muted">{en.deadlines.pageIntro}</p>
      <JobCalendarMonth
        jobs={jobs}
        selected={selected}
        onSelectJob={onSelectJob}
        googleOauthConnected={googleOauthConnected}
        hasManualGoogleToken={hasManualGoogleToken}
        onCreateInGoogle={onCreateInGoogle}
        onOpenSettings={onOpenSettings}
      />
    </section>
  );
});
