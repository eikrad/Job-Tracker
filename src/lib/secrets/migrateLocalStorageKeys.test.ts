import { describe, expect, it, vi } from "vitest";
import { migrateLocalStorageSecrets } from "./migrateLocalStorageKeys";

describe("migrateLocalStorageSecrets", () => {
  it("writes to store, confirms status, then removes localStorage", async () => {
    const store = new Map<string, string>();
    const local = new Map([["geminiApiKey", "gem-secret"]]);
    const setKey = vi.fn(async (provider: string, key: string) => {
      store.set(provider, key);
    });
    const status = vi.fn(async (provider: string) => ({
      configured: store.has(provider),
      backend: "memory",
    }));

    const result = await migrateLocalStorageSecrets({
      getItem: (k) => local.get(k) ?? null,
      removeItem: (k) => {
        local.delete(k);
      },
      setKey,
      status,
    });

    expect(setKey).toHaveBeenCalledWith("gemini", "gem-secret");
    expect(local.has("geminiApiKey")).toBe(false);
    expect(result.migrated).toContain("gemini");
    expect(result.keptLocal).toEqual([]);
  });

  it("keeps localStorage when keyring write fails", async () => {
    const local = new Map([["mistralApiKey", "mis-secret"]]);
    const result = await migrateLocalStorageSecrets({
      getItem: (k) => local.get(k) ?? null,
      removeItem: (k) => {
        local.delete(k);
      },
      setKey: async () => {
        throw new Error("no secret service");
      },
      status: async () => ({ configured: false, backend: "file" }),
    });

    expect(local.get("mistralApiKey")).toBe("mis-secret");
    expect(result.keptLocal).toContain("mistral");
    expect(result.warnings.length).toBeGreaterThan(0);
  });

  it("does not overwrite an already-configured store entry", async () => {
    const local = new Map([["serpApiKey", "stale"]]);
    const setKey = vi.fn(async () => undefined);
    const result = await migrateLocalStorageSecrets({
      getItem: (k) => local.get(k) ?? null,
      removeItem: (k) => {
        local.delete(k);
      },
      setKey,
      status: async () => ({ configured: true, backend: "keyring" }),
    });

    expect(setKey).not.toHaveBeenCalled();
    expect(local.has("serpApiKey")).toBe(false);
    expect(result.migrated).toContain("serpapi");
  });

  it("second run is a no-op when localStorage is empty", async () => {
    const setKey = vi.fn(async () => undefined);
    const result = await migrateLocalStorageSecrets({
      getItem: () => null,
      removeItem: () => undefined,
      setKey,
      status: async () => ({ configured: false, backend: "keyring" }),
    });
    expect(setKey).not.toHaveBeenCalled();
    expect(result.migrated).toEqual([]);
  });
});
