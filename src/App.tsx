import { useCallback, useEffect, useMemo, useState } from "react";
import "./App.css";
import { JobForm } from "./features/jobs/JobForm";
import { JobBoard } from "./features/jobs/JobBoard";
import { JobTable } from "./features/jobs/JobTable";
import { JobDetailTimeline } from "./features/jobs/JobDetailTimeline";
import { DeadlinesView } from "./features/deadlines/DeadlinesView";
import { ReminderCenter } from "./features/reminders/ReminderCenter";
import {
  createJob,
  importJobs,
  initDb,
  listJobs,
  updateJobStatus,
} from "./lib/tauriApi";
import type { Job, NewJob } from "./lib/types";
import { extractJobInfoWithGemini } from "./features/extraction/extractJobInfo";
import {
  exportJobsAsCsv,
  exportJobsAsJson,
  parseJobsImportCsv,
  parseJobsImportJson,
} from "./lib/export/exportBundle";
import { findDuplicateJob } from "./lib/jobs/duplicateCheck";
import { DEFAULT_STATUSES } from "./lib/types";
import { en } from "./i18n/en";

type View = "kanban" | "table" | "calendar";

function App() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [selected, setSelected] = useState<Job | undefined>();
  const [view, setView] = useState<View>("kanban");
  const [geminiApiKey, setGeminiApiKey] = useState(localStorage.getItem("geminiApiKey") ?? "");
  const [googleAccessToken, setGoogleAccessToken] = useState(
    localStorage.getItem("googleAccessToken") ?? "",
  );
  const [statuses, setStatuses] = useState<string[]>(
    JSON.parse(localStorage.getItem("statuses") ?? JSON.stringify(DEFAULT_STATUSES)),
  );

  const hasJobs = useMemo(() => jobs.length > 0, [jobs.length]);

  const refresh = useCallback(async () => {
    const list = await listJobs();
    setJobs(list);
    return list;
  }, []);

  useEffect(() => {
    void initDb().then(() => {
      void refresh();
    });
  }, [refresh]);

  useEffect(() => {
    localStorage.setItem("geminiApiKey", geminiApiKey);
  }, [geminiApiKey]);
  useEffect(() => {
    localStorage.setItem("googleAccessToken", googleAccessToken);
  }, [googleAccessToken]);
  useEffect(() => {
    localStorage.setItem("statuses", JSON.stringify(statuses));
  }, [statuses]);

  const onSubmit = useCallback(
    async (payload: NewJob) => {
      const duplicate = findDuplicateJob(jobs, payload);
      if (duplicate) {
        const proceed = window.confirm(en.alerts.duplicateConfirm);
        if (!proceed) return;
      }
      const id = await createJob(payload);
      const list = await refresh();
      setSelected(list.find((j) => j.id === id));
    },
    [jobs, refresh],
  );

  const onImportFile = useCallback(
    async (file: File | undefined) => {
      if (!file) return;
      const text = await file.text();
      try {
        const rows = file.name.toLowerCase().endsWith(".csv")
          ? parseJobsImportCsv(text)
          : parseJobsImportJson(text);
        if (rows.length === 0) {
          window.alert(en.alerts.importNoRows);
          return;
        }
        const n = await importJobs(rows);
        await refresh();
        window.alert(en.alerts.importCount(n));
      } catch (e) {
        window.alert(en.alerts.importFailed(String(e)));
      }
    },
    [refresh],
  );

  const onMove = useCallback(
    async (jobId: number, status: string) => {
      await updateJobStatus(jobId, status);
      await refresh();
    },
    [refresh],
  );

  const onExtract = useCallback(
    (rawText: string) => extractJobInfoWithGemini(rawText, geminiApiKey),
    [geminiApiKey],
  );

  const renameStatus = useCallback((index: number, value: string) => {
    setStatuses((prev) => {
      const next = [...prev];
      next[index] = value || prev[index];
      return next;
    });
  }, []);

  const moveStatus = useCallback((index: number, direction: -1 | 1) => {
    setStatuses((prev) => {
      const target = index + direction;
      if (target < 0 || target >= prev.length) return prev;
      const next = [...prev];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  }, []);

  /** Narrow return type for child props typed as `Promise<void>`. */
  const syncJobList = useCallback(async () => {
    await refresh();
  }, [refresh]);

  return (
    <main className="app">
      <header className="topBar">
        <h1>{en.app.title}</h1>
        <div className="row">
          <button type="button" onClick={() => setView("kanban")}>
            {en.nav.kanban}
          </button>
          <button type="button" onClick={() => setView("table")}>
            {en.nav.table}
          </button>
          <button type="button" onClick={() => setView("calendar")}>
            {en.nav.calendar}
          </button>
          <button type="button" onClick={() => exportJobsAsJson(jobs)}>
            {en.nav.exportJson}
          </button>
          <button type="button" onClick={() => exportJobsAsCsv(jobs)}>
            {en.nav.exportCsv}
          </button>
          <label className="fileImport">
            <span>{en.nav.importLabel}</span>
            <input
              type="file"
              accept=".json,.csv,application/json,text/csv"
              onChange={(e) => void onImportFile(e.target.files?.[0])}
            />
          </label>
        </div>
      </header>

      <section className="card">
        <label>
          {en.app.geminiKey}
          <input
            value={geminiApiKey}
            onChange={(e) => setGeminiApiKey(e.target.value)}
            placeholder={en.app.geminiPlaceholder}
          />
        </label>
        <label>
          {en.app.googleToken}
          <input
            value={googleAccessToken}
            onChange={(e) => setGoogleAccessToken(e.target.value)}
            placeholder={en.app.googlePlaceholder}
          />
        </label>
      </section>

      <section className="card">
        <h2>{en.app.statusColumns}</h2>
        {statuses.map((status, index) => (
          <div className="row" key={`${status}-${index}`}>
            <input value={status} onChange={(e) => renameStatus(index, e.target.value)} />
            <button type="button" onClick={() => moveStatus(index, -1)}>
              {en.app.up}
            </button>
            <button type="button" onClick={() => moveStatus(index, 1)}>
              {en.app.down}
            </button>
          </div>
        ))}
      </section>

      <JobForm statuses={statuses} onSubmit={onSubmit} onExtract={onExtract} />

      {view === "kanban" && (
        <JobBoard statuses={statuses} jobs={jobs} onMove={onMove} onSelect={setSelected} />
      )}
      {view === "table" && <JobTable jobs={jobs} onSelect={setSelected} />}
      {view === "calendar" && (
        <DeadlinesView
          jobs={jobs}
          googleAccessToken={googleAccessToken}
          onApiSynced={syncJobList}
        />
      )}

      <ReminderCenter jobs={jobs} />
      <JobDetailTimeline selected={selected} onSavedPdf={syncJobList} />
      {!hasJobs && <p>{en.app.noJobsYet}</p>}
    </main>
  );
}

export default App;
