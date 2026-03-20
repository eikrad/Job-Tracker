import { useCallback, useEffect, useState } from "react";
import {
  createJob,
  importJobs,
  initDb,
  listJobs,
  updateJobStatus,
} from "../lib/tauriApi";
import type { Job, NewJob } from "../lib/types";
import { extractJobInfo, type LlmProvider } from "../features/extraction/extractJobInfo";
import { parseJobsImportCsv, parseJobsImportJson } from "../lib/export/exportBundle";
import { findDuplicateJob } from "../lib/jobs/duplicateCheck";
import { DEFAULT_STATUSES } from "../lib/types";
import { en } from "../i18n/en";

type BoardView = "kanban" | "table" | "calendar";

function readLlmProvider(): LlmProvider {
  const p = localStorage.getItem("llmProvider");
  return p === "mistral" ? "mistral" : "gemini";
}

export function useJobTrackerState() {
  const [jobs, setJobs] = useState<Job[]>([]);
  const [selected, setSelected] = useState<Job | undefined>();
  const [view, setView] = useState<BoardView>("kanban");
  const [llmProvider, setLlmProvider] = useState<LlmProvider>(readLlmProvider);
  const [geminiApiKey, setGeminiApiKey] = useState(localStorage.getItem("geminiApiKey") ?? "");
  const [mistralApiKey, setMistralApiKey] = useState(localStorage.getItem("mistralApiKey") ?? "");
  const [googleAccessToken, setGoogleAccessToken] = useState(
    localStorage.getItem("googleAccessToken") ?? "",
  );
  const [statuses, setStatuses] = useState<string[]>(
    JSON.parse(localStorage.getItem("statuses") ?? JSON.stringify(DEFAULT_STATUSES)),
  );

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
    localStorage.setItem("llmProvider", llmProvider);
  }, [llmProvider]);
  useEffect(() => {
    localStorage.setItem("geminiApiKey", geminiApiKey);
  }, [geminiApiKey]);
  useEffect(() => {
    localStorage.setItem("mistralApiKey", mistralApiKey);
  }, [mistralApiKey]);
  useEffect(() => {
    localStorage.setItem("googleAccessToken", googleAccessToken);
  }, [googleAccessToken]);
  useEffect(() => {
    localStorage.setItem("statuses", JSON.stringify(statuses));
  }, [statuses]);

  const onSubmit = useCallback(
    async (payload: NewJob): Promise<boolean> => {
      const duplicate = findDuplicateJob(jobs, payload);
      if (duplicate) {
        const proceed = window.confirm(en.alerts.duplicateConfirm);
        if (!proceed) return false;
      }
      const id = await createJob(payload);
      const list = await refresh();
      setSelected(list.find((j) => j.id === id));
      return true;
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
    (rawText: string) =>
      extractJobInfo(rawText, llmProvider, llmProvider === "gemini" ? geminiApiKey : mistralApiKey),
    [llmProvider, geminiApiKey, mistralApiKey],
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

  const syncJobList = useCallback(async () => {
    await refresh();
  }, [refresh]);

  return {
    jobs,
    selected,
    setSelected,
    view,
    setView,
    llmProvider,
    setLlmProvider,
    geminiApiKey,
    setGeminiApiKey,
    mistralApiKey,
    setMistralApiKey,
    googleAccessToken,
    setGoogleAccessToken,
    statuses,
    onSubmit,
    onImportFile,
    onMove,
    onExtract,
    renameStatus,
    moveStatus,
    syncJobList,
  };
}

export type JobTrackerState = ReturnType<typeof useJobTrackerState>;
