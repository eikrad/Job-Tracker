import { describe, expect, it, vi } from "vitest";
import { extractJobInfo, toJobPartial } from "./extractJobInfo";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

describe("toJobPartial", () => {
  it("keeps known string fields", () => {
    expect(toJobPartial({ company: "Acme", title: "Dev" })).toEqual({
      company: "Acme",
      title: "Dev",
    });
  });

  it("trims values and drops blank ones", () => {
    expect(toJobPartial({ company: "  Acme  ", title: "   " })).toEqual({ company: "Acme" });
  });

  it("drops priority — manual only, never from an LLM", () => {
    const result = toJobPartial({ priority: 2, company: "Acme" });
    expect(result).not.toHaveProperty("priority");
    expect(result.company).toBe("Acme");
  });

  it("drops unknown keys and non-string values", () => {
    expect(toJobPartial({ company: "Acme", nonsense: "x", tags: ["a", "b"] })).toEqual({
      company: "Acme",
    });
  });
});

describe("extractJobInfo", () => {
  it("returns error when text empty", async () => {
    await expect(extractJobInfo("", "mistral")).resolves.toMatchObject({
      ok: false,
      error: expect.stringContaining("Paste job ad"),
    });
  });

  it("invokes Rust extract_job_info and maps partial", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      ok: true,
      partial: { company: "MCo", title: "Eng" },
    });
    const out = await extractJobInfo("some ad text", "mistral");
    expect(out).toEqual({ ok: true, partial: { company: "MCo", title: "Eng" } });
    expect(invoke).toHaveBeenCalledWith("extract_job_info", {
      rawText: "some ad text",
      provider: "mistral",
    });
  });

  it("reports a parse failure when nothing usable came back", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ ok: true, partial: { unknown: "x" } });
    await expect(extractJobInfo("text", "mistral")).resolves.toMatchObject({
      ok: false,
      error: expect.stringContaining("Could not parse JSON"),
    });
  });

  it("surfaces Rust error", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      ok: false,
      error: "Add an API key in Settings (Job Tracker).",
    });
    const out = await extractJobInfo("text", "gemini");
    expect(out).toEqual({
      ok: false,
      error: "Add an API key in Settings (Job Tracker).",
    });
  });

  it("explains that extraction needs the desktop app when Tauri is absent", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("invoke is not available"));
    await expect(extractJobInfo("text", "gemini")).resolves.toMatchObject({
      ok: false,
      error: expect.stringContaining("desktop app"),
    });
  });
});
