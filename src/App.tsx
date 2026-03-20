import { useEffect, useMemo, useState } from "react";
import "./App.css";
import { JobForm } from "./features/jobs/JobForm";
import { JobBoard } from "./features/jobs/JobBoard";
import { JobTable } from "./features/jobs/JobTable";
import { JobDetailTimeline } from "./features/jobs/JobDetailTimeline";
import { DeadlinesView } from "./features/deadlines/DeadlinesView";
import { ReminderCenter } from "./features/reminders/ReminderCenter";
import { createJob, initDb, listJobs, updateJobStatus } from "./lib/tauriApi";
import type { Job, NewJob } from "./lib/types";
import { extractJobInfoWithGemini } from "./features/extraction/extractJobInfo";
import { exportJobsAsJson } from "./lib/export/exportBundle";
import { DEFAULT_STATUSES } from "./lib/types";

type View = "kanban" | "table" | "calendar";

function App() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [selected, setSelected] = useState<Job | undefined>();
  const [view, setView] = useState<View>("kanban");
  const [geminiApiKey, setGeminiApiKey] = useState(localStorage.getItem("geminiApiKey") ?? "");
  const [statuses, setStatuses] = useState<string[]>(
    JSON.parse(localStorage.getItem("statuses") ?? JSON.stringify(DEFAULT_STATUSES)),
  );

  const hasJobs = useMemo(() => jobs.length > 0, [jobs.length]);

  async function refresh() {
    setJobs(await listJobs());
  }

  useEffect(() => {
    void initDb().then(refresh);
  }, []);

  useEffect(() => {
    localStorage.setItem("geminiApiKey", geminiApiKey);
  }, [geminiApiKey]);
  useEffect(() => {
    localStorage.setItem("statuses", JSON.stringify(statuses));
  }, [statuses]);

  async function onSubmit(payload: NewJob) {
    const duplicate = jobs.find(
      (j) =>
        (!!payload.url && !!j.url && payload.url === j.url) ||
        (j.company.toLowerCase() === payload.company.toLowerCase() &&
          (j.title ?? "").toLowerCase() === (payload.title ?? "").toLowerCase()),
    );
    if (duplicate) {
      const proceed = window.confirm("Possible duplicate found. Save anyway?");
      if (!proceed) return;
    }
    const id = await createJob(payload);
    await refresh();
    setSelected(jobs.find((j) => j.id === id));
  }

  async function onMove(jobId: number, status: string) {
    await updateJobStatus(jobId, status);
    await refresh();
  }

  async function onExtract(rawText: string) {
    return extractJobInfoWithGemini(rawText, geminiApiKey);
  }

  function renameStatus(index: number, value: string) {
    setStatuses((prev) => {
      const next = [...prev];
      next[index] = value || prev[index];
      return next;
    });
  }

  function moveStatus(index: number, direction: -1 | 1) {
    setStatuses((prev) => {
      const target = index + direction;
      if (target < 0 || target >= prev.length) return prev;
      const next = [...prev];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  }

  return (
    <main className="app">
      <header className="topBar">
        <h1>Job Tracker</h1>
        <div className="row">
          <button onClick={() => setView("kanban")}>Kanban</button>
          <button onClick={() => setView("table")}>Table</button>
          <button onClick={() => setView("calendar")}>Calendar</button>
          <button onClick={() => exportJobsAsJson(jobs)}>Export JSON</button>
        </div>
      </header>

      <section className="card">
        <label>
          Gemini API Key
          <input
            value={geminiApiKey}
            onChange={(e) => setGeminiApiKey(e.target.value)}
            placeholder="Paste your Gemini key"
          />
        </label>
      </section>

      <section className="card">
        <h2>Status Columns (rename/reorder)</h2>
        {statuses.map((status, index) => (
          <div className="row" key={`${status}-${index}`}>
            <input value={status} onChange={(e) => renameStatus(index, e.target.value)} />
            <button onClick={() => moveStatus(index, -1)}>Up</button>
            <button onClick={() => moveStatus(index, 1)}>Down</button>
          </div>
        ))}
      </section>

      <JobForm statuses={statuses} onSubmit={onSubmit} onExtract={onExtract} />

      {view === "kanban" && <JobBoard statuses={statuses} jobs={jobs} onMove={onMove} onSelect={setSelected} />}
      {view === "table" && <JobTable jobs={jobs} onSelect={setSelected} />}
      {view === "calendar" && <DeadlinesView jobs={jobs} />}

      <ReminderCenter jobs={jobs} />
      <JobDetailTimeline selected={selected} onSavedPdf={refresh} />
      {!hasJobs && <p>No jobs yet.</p>}
    </main>
  );
}

export default App;
