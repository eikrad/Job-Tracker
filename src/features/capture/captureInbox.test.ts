import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  enqueueBrowserCaptureUrl,
  listCaptureInboxItems,
  updateCaptureInboxItem,
} from "./captureInbox";

describe("captureInbox", () => {
  beforeEach(() => {
    const store = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: vi.fn((key: string) => store.get(key) ?? null),
      setItem: vi.fn((key: string, value: string) => {
        store.set(key, value);
      }),
      removeItem: vi.fn((key: string) => {
        store.delete(key);
      }),
      clear: vi.fn(() => {
        store.clear();
      }),
    });
  });

  it("enqueues browser URLs and prevents duplicates", () => {
    const first = enqueueBrowserCaptureUrl("https://example.com/job");
    const second = enqueueBrowserCaptureUrl("https://example.com/job");
    expect(first.id).toBe(second.id);
    expect(listCaptureInboxItems()).toHaveLength(1);
  });

  it("updates item status and warning", () => {
    const item = enqueueBrowserCaptureUrl("https://example.com/job");
    updateCaptureInboxItem(item.id, { status: "ready", warning: "partial" });
    const updated = listCaptureInboxItems()[0];
    expect(updated.status).toBe("ready");
    expect(updated.warning).toBe("partial");
  });
});
