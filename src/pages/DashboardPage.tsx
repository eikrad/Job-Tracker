import { JobBoard } from "../features/jobs/JobBoard";
import { JobTable } from "../features/jobs/JobTable";
import { JobDetailTimeline } from "../features/jobs/JobDetailTimeline";
import { DeadlinesView } from "../features/deadlines/DeadlinesView";
import { ReminderCenter } from "../features/reminders/ReminderCenter";
import { useJobTracker } from "../context/JobTrackerContext";

export function DashboardPage() {
  const {
    jobs,
    selected,
    setSelected,
    view,
    statuses,
    googleAccessToken,
    onMove,
    onDeleteJob,
    onUpdateJob,
    onExtract,
    syncJobList,
    runBackup,
    googleOauthConnected,
    createGoogleCalendarEvent,
    openSettings,
  } = useJobTracker();

  return (
    <div className="appLayout">
      <div className="appMain">
        {view === "kanban" && (
          <JobBoard statuses={statuses} jobs={jobs} onMove={onMove} onSelect={setSelected} />
        )}
        {view === "table" && <JobTable jobs={jobs} onSelect={setSelected} />}
        {view === "calendar" && (
          <DeadlinesView
            jobs={jobs}
            selected={selected}
            onSelectJob={setSelected}
            googleOauthConnected={googleOauthConnected}
            hasManualGoogleToken={!!googleAccessToken.trim()}
            onCreateInGoogle={createGoogleCalendarEvent}
            onOpenSettings={openSettings}
          />
        )}
      </div>
      <aside className="appAside">
        <ReminderCenter jobs={jobs} />
        <JobDetailTimeline
          selected={selected}
          onSavedPdf={async () => { await syncJobList(); runBackup(); }}
          onDeleteJob={onDeleteJob}
          statuses={statuses}
          onExtract={onExtract}
          onUpdateJob={onUpdateJob}
        />
      </aside>
    </div>
  );
}
