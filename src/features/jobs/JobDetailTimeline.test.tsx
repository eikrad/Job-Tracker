// @vitest-environment happy-dom
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Job } from "../../lib/types";
import { JobDetailTimeline } from "./JobDetailTimeline";

vi.mock("../../lib/tauriApi", () => ({
  checkListingStatus: vi.fn(),
  openUrlInBrowser: vi.fn().mockResolvedValue(undefined),
}));

import { openUrlInBrowser } from "../../lib/tauriApi";

const job: Job = {
  id: 7,
  company: "Acme",
  title: "Engineer",
  url: "https://example.com/jobs/7",
  raw_text: null,
  status: "Interesting",
  deadline: null,
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
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
};

describe("JobDetailTimeline", () => {
  it("opens the listing URL via the Tauri browser command", () => {
    render(
      <JobDetailTimeline
        selected={job}
        onDeleteJob={vi.fn()}
        onViewDetails={vi.fn()}
        onListingStatusChecked={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTitle("https://example.com/jobs/7"));
    expect(openUrlInBrowser).toHaveBeenCalledWith("https://example.com/jobs/7");
  });
});
