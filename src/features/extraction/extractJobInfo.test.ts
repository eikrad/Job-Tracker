import { describe, expect, it, vi } from "vitest";
import { extractJobInfo, normalizeLlmJobPartial, parsePartialNewJobFromLlmText } from "./extractJobInfo";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

describe("parsePartialNewJobFromLlmText", () => {
  it("parses plain JSON", () => {
    const out = parsePartialNewJobFromLlmText('{"company":"Acme","title":"Dev"}');
    expect(out).toEqual({ company: "Acme", title: "Dev" });
  });

  it("strips markdown fences", () => {
    const out = parsePartialNewJobFromLlmText('```json\n{"company":"X"}\n```');
    expect(out).toEqual({ company: "X" });
  });

  it("returns {} on invalid JSON", () => {
    expect(parsePartialNewJobFromLlmText("not json")).toEqual({});
  });

  it("normalizes Company and Title casings", () => {
    const out = parsePartialNewJobFromLlmText('{"Company":"Acme GmbH","Title":"Dev"}');
    expect(out).toEqual({ company: "Acme GmbH", title: "Dev" });
  });
});

describe("normalizeLlmJobPartial", () => {
  it("maps employer alias to company", () => {
    expect(normalizeLlmJobPartial({ employer: "X" })).toEqual({ company: "X" });
  });

  it("does not map priority (manual only)", () => {
    const result = normalizeLlmJobPartial({ priority: 2, company: "Acme" });
    expect(result).not.toHaveProperty("priority");
    expect(result.company).toBe("Acme");
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
});
