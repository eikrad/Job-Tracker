import { describe, expect, it, vi } from "vitest";
import {
  DEFAULT_BOARD_VIEW,
  loadDefaultBoardView,
  normalizeBoardView,
  saveDefaultBoardView,
} from "./boardViewPreference";

describe("boardViewPreference", () => {
  it("normalizes invalid values to the default table view", () => {
    expect(normalizeBoardView("kanban")).toBe("kanban");
    expect(normalizeBoardView("table")).toBe("table");
    expect(normalizeBoardView("calendar")).toBe("calendar");
    expect(normalizeBoardView("list")).toBe(DEFAULT_BOARD_VIEW);
    expect(normalizeBoardView(null)).toBe(DEFAULT_BOARD_VIEW);
    expect(normalizeBoardView(undefined)).toBe(DEFAULT_BOARD_VIEW);
  });

  it("loads the default when localStorage is empty", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: vi.fn(),
    });
    expect(loadDefaultBoardView()).toBe(DEFAULT_BOARD_VIEW);
  });

  it("loads a valid stored view", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => "calendar",
      setItem: vi.fn(),
    });
    expect(loadDefaultBoardView()).toBe("calendar");
  });

  it("saves the preferred view", () => {
    const setItem = vi.fn();
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem,
    });
    saveDefaultBoardView("kanban");
    expect(setItem).toHaveBeenCalledWith("jobtracker.defaultBoardView", "kanban");
  });
});
