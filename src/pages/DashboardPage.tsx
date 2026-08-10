import { useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { JobBoard } from "../features/jobs/JobBoard";
import { JobTable } from "../features/jobs/JobTable";
import { JobDetailTimeline } from "../features/jobs/JobDetailTimeline";
import { DeadlinesView } from "../features/deadlines/DeadlinesView";
import { ReminderCenter } from "../features/reminders/ReminderCenter";
import { WorkspaceEmpty } from "../components/WorkspaceEmpty";
import { useJobTracker } from "../context/JobTrackerContext";
import { filterJobsBySearch } from "../lib/jobs/filterJobs";
import { en } from "../i18n/en";

export function DashboardPage() {
  const navigate = useNavigate();
  const {
    jobs,
    selected,
    setSelected,
    view,
    jobSearchQuery,
    setJobSearchQuery,
    statuses,
    googleAccessToken,
    onMove,
    onDeleteJob,
    onListingStatusChecked,
    serpApiKey,
    googleOauthConnected,
    createGoogleCalendarEvent,
    openSettings,
  } = useJobTracker();

  const filteredJobs = useMemo(
    () => filterJobsBySearch(jobs, jobSearchQuery),
    [jobs, jobSearchQuery],
  );

  const searchHasNoMatches =
    jobs.length > 0 && filteredJobs.length === 0 && jobSearchQuery.trim() !== "";

  return (
    <div className="appLayout">
      <div className="appMain">
        {searchHasNoMatches ? (
          <section className="card">
            <WorkspaceEmpty
              title={en.empty.searchTitle}
              body={en.empty.searchBody}
              action={{
                label: en.app.jobSearchClear,
                onClick: () => setJobSearchQuery(""),
              }}
            />
          </section>
        ) : (
          <>
            {view === "kanban" && (
              <JobBoard
                statuses={statuses}
                jobs={filteredJobs}
                onMove={onMove}
                onSelect={setSelected}
              />
            )}
            {view === "table" && (
              <JobTable jobs={filteredJobs} statuses={statuses} onSelect={setSelected} />
            )}
            {view === "calendar" && (
              <DeadlinesView
                jobs={filteredJobs}
                selected={selected}
                onSelectJob={setSelected}
                googleOauthConnected={googleOauthConnected}
                hasManualGoogleToken={!!googleAccessToken.trim()}
                onCreateInGoogle={createGoogleCalendarEvent}
                onOpenSettings={openSettings}
              />
            )}
          </>
        )}
      </div>
      <aside className="appAside">
        <JobDetailTimeline
          selected={selected}
          onDeleteJob={onDeleteJob}
          onViewDetails={(id) => navigate(`/job/${id}`)}
          onListingStatusChecked={onListingStatusChecked}
          serpApiKey={serpApiKey}
        />
        <ReminderCenter jobs={jobs} />
      </aside>
    </div>
  );
}
