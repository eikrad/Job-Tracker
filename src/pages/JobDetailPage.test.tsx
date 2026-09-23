// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import { JobTrackerProvider } from "../context/JobTrackerContext";
import type { JobTrackerState } from "../hooks/useJobTrackerState";
import type { Job } from "../lib/types";
import { openUrlInBrowser } from "../lib/tauriApi";
import { JobDetailPage } from "./JobDetailPage";

vi.mock("../lib/tauriApi", () => ({
  checkListingStatus: vi.fn(),
  deleteJobDocument: vi.fn(),
  listJobDocuments: vi.fn().mockResolvedValue([]),
  listStatusHistory: vi.fn().mockResolvedValue([]),
  openDocument: vi.fn(),
  openUrlInBrowser: vi.fn().mockResolvedValue(undefined),
  saveJobDocument: vi.fn(),
}));

const job: Job = {
  id: 7,
  company: "Acme",
  title: "Geodata Analyst",
  url: "https://candidate.hr-manager.net/ad/123",
  board_url: "https://www.jobindex.dk/c?t=h1000001",
  status: "Interesting",
  created_at: "2026-09-20T00:00:00Z",
  updated_at: "2026-09-20T00:00:00Z",
};

function renderDetail(j: Job) {
  const state = {
    jobs: [j],
    statuses: ["Interesting"],
    onDeleteJob: vi.fn(),
    onUpdateJob: vi.fn(),
    onExtract: vi.fn(),
    syncJobList: vi.fn(),
    runBackup: vi.fn(),
    onListingStatusChecked: vi.fn(),
  } as unknown as JobTrackerState;
  return render(
    <MemoryRouter initialEntries={[`/jobs/${j.id}`]}>
      <JobTrackerProvider value={state}>
        <Routes>
          <Route path="/jobs/:id" element={<JobDetailPage />} />
        </Routes>
      </JobTrackerProvider>
    </MemoryRouter>,
  );
}

afterEach(cleanup);

describe("JobDetailPage board link", () => {
  it("shows the board the job was found through next to the employer ad", () => {
    renderDetail(job);

    const via = screen.getByRole("link", { name: "via jobindex.dk" });
    fireEvent.click(via);

    expect(openUrlInBrowser).toHaveBeenCalledWith("https://www.jobindex.dk/c?t=h1000001");
  });

  it("shows no board link when the job has none", () => {
    renderDetail({ ...job, board_url: null });

    expect(screen.queryByText(/^via /)).toBeNull();
  });
});
