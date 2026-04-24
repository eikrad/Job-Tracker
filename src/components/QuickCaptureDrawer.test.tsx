// @vitest-environment happy-dom
import { fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { useState, type ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { JobTrackerProvider } from "../context/JobTrackerContext";
import type { JobTrackerState } from "../hooks/useJobTrackerState";
import { QuickCaptureDrawer } from "./QuickCaptureDrawer";

function makeTrackerState(): JobTrackerState {
  return {
    jobs: [],
    selected: undefined,
    setSelected: vi.fn(),
    view: "table",
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
    statuses: ["Interesting", "Application Sent"],
    backupFolder: "",
    setBackupFolder: vi.fn(),
    runBackup: vi.fn(),
    onSubmit: vi.fn().mockResolvedValue(true),
    onImportFile: vi.fn(),
    onMove: vi.fn(),
    onDeleteJob: vi.fn(),
    onUpdateJob: vi.fn(),
    onExtract: vi.fn().mockResolvedValue({ ok: false, error: "fallback" }),
    renameStatus: vi.fn(),
    moveStatus: vi.fn(),
    syncJobList: vi.fn(),
  };
}

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <MemoryRouter>
      <JobTrackerProvider value={makeTrackerState()}>{children}</JobTrackerProvider>
    </MemoryRouter>
  );
}

function DrawerHarness() {
  const [open, setOpen] = useState(true);
  return <QuickCaptureDrawer open={open} onClose={() => setOpen(false)} />;
}

describe("QuickCaptureDrawer", () => {
  it("opens and closes on escape", () => {
    render(<DrawerHarness />, { wrapper: Wrapper });
    expect(screen.getByRole("dialog", { name: "Quick capture drawer" })).not.toBeNull();
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "Quick capture drawer" })).toBeNull();
  });
});
