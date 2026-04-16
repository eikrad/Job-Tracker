// @vitest-environment happy-dom
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { JobTrackerProvider } from "../context/JobTrackerContext";
import type { JobTrackerState } from "../hooks/useJobTrackerState";
import type { Job } from "../lib/types";
import { DashboardPage } from "./DashboardPage";

const baseJob: Job = {
  id: 1,
  company: "Acme",
  title: "Developer",
  url: null,
  raw_text: null,
  status: "Interesting",
  deadline: "2026-04-20",
  interview_date: null,
  start_date: null,
  tags: null,
  detected_language: null,
  notes: null,
  contact_name: null,
  contact_email: null,
  contact_phone: null,
  workplace_street: null,
  workplace_city: null,
  workplace_postal_code: null,
  work_mode: null,
  salary_range: null,
  contract_type: null,
  priority: null,
  reference_number: null,
  source: null,
  pdf_path: null,
  created_at: "2026-04-16T00:00:00Z",
  updated_at: "2026-04-16T00:00:00Z",
};

function makeTrackerState(view: JobTrackerState["view"]): JobTrackerState {
  return {
    jobs: [baseJob],
    selected: baseJob,
    setSelected: vi.fn(),
    view,
    setView: vi.fn(),
    llmProvider: "gemini",
    setLlmProvider: vi.fn(),
    geminiApiKey: "",
    setGeminiApiKey: vi.fn(),
    mistralApiKey: "",
    setMistralApiKey: vi.fn(),
    serpApiKey: "",
    setSerpApiKey: vi.fn(),
    braveSearchApiKey: "",
    setBraveSearchApiKey: vi.fn(),
    googleAccessToken: "",
    setGoogleAccessToken: vi.fn(),
    googleOauthConnected: false,
    refreshGoogleOauthStatus: vi.fn(),
    connectGoogleCalendar: vi.fn(),
    disconnectGoogleCalendar: vi.fn(),
    createGoogleCalendarEvent: vi.fn(),
    openSettings: vi.fn(),
    statuses: ["Interesting", "Application Sent", "Feedback", "Done"],
    backupFolder: "",
    setBackupFolder: vi.fn(),
    runBackup: vi.fn(),
    onSubmit: vi.fn(),
    onImportFile: vi.fn(),
    onMove: vi.fn(),
    onDeleteJob: vi.fn(),
    onUpdateJob: vi.fn(),
    onExtract: vi.fn(),
    renameStatus: vi.fn(),
    moveStatus: vi.fn(),
    syncJobList: vi.fn(),
  };
}

function renderDashboard(view: JobTrackerState["view"]) {
  return render(
    <MemoryRouter>
      <JobTrackerProvider value={makeTrackerState(view)}>
        <DashboardPage />
      </JobTrackerProvider>
    </MemoryRouter>,
  );
}

describe("DashboardPage", () => {
  it("renders scrollable wrappers for reminders, detail panel, and kanban lanes", () => {
    renderDashboard("kanban");

    expect(document.querySelector(".dashboardPanelScroll")).not.toBeNull();
    expect(document.querySelector(".columnBody")).not.toBeNull();
  });

  it("keeps table content rendering while sidebar panels use dashboard hooks", () => {
    renderDashboard("table");

    expect(document.querySelector(".appAside .dashboardPanel")).not.toBeNull();
    expect(screen.getAllByText("Acme").length).toBeGreaterThan(0);
  });
});
