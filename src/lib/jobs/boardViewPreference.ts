import { readStoredString, writeStoredString } from "../storage/localStoragePref";

export type BoardView = "kanban" | "table" | "calendar";

export const DEFAULT_BOARD_VIEW: BoardView = "table";

export const BOARD_VIEWS: BoardView[] = ["kanban", "table", "calendar"];

const STORAGE_KEY = "jobtracker.defaultBoardView";

export function normalizeBoardView(raw: unknown): BoardView {
  if (raw === "kanban" || raw === "table" || raw === "calendar") return raw;
  return DEFAULT_BOARD_VIEW;
}

export function loadDefaultBoardView(): BoardView {
  return normalizeBoardView(readStoredString(STORAGE_KEY));
}

export function saveDefaultBoardView(view: BoardView): void {
  writeStoredString(STORAGE_KEY, normalizeBoardView(view));
}
