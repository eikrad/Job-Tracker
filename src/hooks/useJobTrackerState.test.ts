// @vitest-environment happy-dom
import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useJobTrackerState } from "./useJobTrackerState";

vi.mock("../lib/tauriApi", () => ({
  initDb: vi.fn().mockResolvedValue(undefined),
  listJobs: vi.fn().mockResolvedValue([]),
  googleOauthStatus: vi.fn().mockResolvedValue({ connected: false }),
  backupToFolder: vi.fn(),
  createJob: vi.fn(),
  deleteJob: vi.fn(),
  googleCalendarCreateEvent: vi.fn(),
  googleOauthConnect: vi.fn(),
  googleOauthDisconnect: vi.fn(),
  importJobs: vi.fn(),
  updateJob: vi.fn(),
  updateJobStatus: vi.fn(),
}));

describe("useJobTrackerState", () => {
  beforeEach(() => {
    const storage = {
      getItem: vi.fn(() => null),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    };
    vi.stubGlobal("localStorage", storage);
  });

  it("defaults the dashboard view to table", () => {
    const { result } = renderHook(() => useJobTrackerState());
    expect(result.current.view).toBe("table");
  });
});
