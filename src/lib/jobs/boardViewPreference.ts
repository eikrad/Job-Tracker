export type BoardView = "kanban" | "table" | "calendar";

export const DEFAULT_BOARD_VIEW: BoardView = "table";

export const BOARD_VIEWS: BoardView[] = ["kanban", "table", "calendar"];

const STORAGE_KEY = "jobtracker.defaultBoardView";

function storageAvailable(): boolean {
  try {
    return typeof localStorage !== "undefined" && typeof localStorage.setItem === "function";
  } catch {
    return false;
  }
}

export function normalizeBoardView(raw: unknown): BoardView {
  if (raw === "kanban" || raw === "table" || raw === "calendar") return raw;
  return DEFAULT_BOARD_VIEW;
}

export function loadDefaultBoardView(): BoardView {
  if (!storageAvailable()) return DEFAULT_BOARD_VIEW;
  try {
    return normalizeBoardView(localStorage.getItem(STORAGE_KEY));
  } catch {
    return DEFAULT_BOARD_VIEW;
  }
}

export function saveDefaultBoardView(view: BoardView): void {
  if (!storageAvailable()) return;
  try {
    localStorage.setItem(STORAGE_KEY, normalizeBoardView(view));
  } catch {
    /* ignore quota / private mode */
  }
}
