import { describe, expect, it, vi } from "vitest";
import {
  readStoredJson,
  readStoredString,
  storageAvailable,
  writeStoredJson,
  writeStoredString,
} from "./localStoragePref";

describe("localStoragePref", () => {
  it("reports storage unavailable when localStorage is missing", () => {
    vi.stubGlobal("localStorage", undefined);
    expect(storageAvailable()).toBe(false);
  });

  it("reads and writes strings", () => {
    const setItem = vi.fn();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => (key === "k" ? "v" : null),
      setItem,
    });
    expect(storageAvailable()).toBe(true);
    expect(readStoredString("k")).toBe("v");
    writeStoredString("k", "next");
    expect(setItem).toHaveBeenCalledWith("k", "next");
  });

  it("swallows write failures", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => null,
      setItem: () => {
        throw new Error("quota");
      },
    });
    expect(() => writeStoredString("k", "v")).not.toThrow();
  });

  it("reads and writes JSON", () => {
    const setItem = vi.fn();
    vi.stubGlobal("localStorage", {
      getItem: () => JSON.stringify(["Done"]),
      setItem,
    });
    expect(readStoredJson("hidden")).toEqual(["Done"]);
    writeStoredJson("hidden", ["Interesting"]);
    expect(setItem).toHaveBeenCalledWith("hidden", JSON.stringify(["Interesting"]));
  });

  it("returns null for invalid JSON", () => {
    vi.stubGlobal("localStorage", {
      getItem: () => "{not-json",
      setItem: vi.fn(),
    });
    expect(readStoredJson("bad")).toBeNull();
  });
});
