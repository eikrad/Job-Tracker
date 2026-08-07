// @vitest-environment happy-dom
import { act, renderHook } from "@testing-library/react";
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
    expect(result.current.defaultBoardView).toBe("table");
  });

  it("initializes the dashboard view from the saved default preference", () => {
    const storage = {
      getItem: vi.fn((key: string) => (key === "jobtracker.defaultBoardView" ? "kanban" : null)),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    };
    vi.stubGlobal("localStorage", storage);
    const { result } = renderHook(() => useJobTrackerState());
    expect(result.current.view).toBe("kanban");
    expect(result.current.defaultBoardView).toBe("kanban");
  });

  it("persists default board view without treating tab switches as the default", () => {
    const setItem = vi.fn();
    const storage = {
      getItem: vi.fn(() => null),
      setItem,
      removeItem: vi.fn(),
      clear: vi.fn(),
    };
    vi.stubGlobal("localStorage", storage);
    const { result } = renderHook(() => useJobTrackerState());

    act(() => {
      result.current.setView("calendar");
    });
    expect(result.current.view).toBe("calendar");
    expect(result.current.defaultBoardView).toBe("table");
    expect(setItem).not.toHaveBeenCalledWith("jobtracker.defaultBoardView", expect.anything());

    act(() => {
      result.current.setDefaultBoardView("kanban");
    });
    expect(result.current.defaultBoardView).toBe("kanban");
    expect(result.current.view).toBe("kanban");
    expect(setItem).toHaveBeenCalledWith("jobtracker.defaultBoardView", "kanban");
  });

  it("migrates saved statuses from v1 to v2 on init", () => {
    const storage = {
      getItem: vi.fn((key: string) =>
        key === "statuses"
          ? JSON.stringify(["Interesting", "Application Sent", "Feedback", "Done"])
          : null,
      ),
      setItem: vi.fn(),
      removeItem: vi.fn(),
      clear: vi.fn(),
    };
    vi.stubGlobal("localStorage", storage);
    const { result } = renderHook(() => useJobTrackerState());
    expect(result.current.statuses).toContain("Plan to Apply");
    expect(result.current.statuses.indexOf("Plan to Apply")).toBe(1);
  });
});
