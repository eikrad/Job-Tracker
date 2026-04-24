import type { NewJob } from "../../lib/types";

export type CaptureInboxStatus = "pending" | "ready" | "accepted" | "dismissed" | "failed";

export type CaptureInboxItem = {
  id: string;
  url: string;
  channel: "browser_inbox";
  status: CaptureInboxStatus;
  createdAt: string;
  updatedAt: string;
  warning?: string;
  draft?: NewJob;
};

const STORAGE_KEY = "captureInboxItems";

function nowIso() {
  return new Date().toISOString();
}

function readRaw(): CaptureInboxItem[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? (parsed as CaptureInboxItem[]) : [];
  } catch {
    return [];
  }
}

function writeRaw(items: CaptureInboxItem[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(items));
}

export function listCaptureInboxItems(): CaptureInboxItem[] {
  return readRaw().sort((a, b) => b.createdAt.localeCompare(a.createdAt));
}

export function enqueueBrowserCaptureUrl(url: string): CaptureInboxItem {
  const items = readRaw();
  const existing = items.find((item) => item.url === url && item.status !== "dismissed");
  if (existing) return existing;
  const item: CaptureInboxItem = {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    url,
    channel: "browser_inbox",
    status: "pending",
    createdAt: nowIso(),
    updatedAt: nowIso(),
  };
  writeRaw([item, ...items]);
  return item;
}

export function updateCaptureInboxItem(id: string, patch: Partial<CaptureInboxItem>) {
  const items = readRaw().map((item) =>
    item.id === id
      ? {
          ...item,
          ...patch,
          updatedAt: nowIso(),
        }
      : item,
  );
  writeRaw(items);
}
