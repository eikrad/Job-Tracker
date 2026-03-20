import { describe, expect, it, vi } from "vitest";
import { extractJobInfo, normalizeLlmJobPartial, parsePartialNewJobFromLlmText } from "./extractJobInfo";

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
});

describe("extractJobInfo", () => {
  it("returns error when key or text empty", async () => {
    await expect(extractJobInfo("hello", "mistral", "")).resolves.toMatchObject({
      ok: false,
      error: expect.stringContaining("API key"),
    });
    await expect(extractJobInfo("", "mistral", "k")).resolves.toMatchObject({
      ok: false,
      error: expect.stringContaining("Paste job ad"),
    });
  });

  it("calls Mistral API and maps response", async () => {
    const payload = { choices: [{ message: { content: '{"company":"MCo","title":"Eng"}' } }] };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify(payload), { status: 200, headers: { "Content-Type": "application/json" } }),
    );
    const out = await extractJobInfo("some ad text", "mistral", "test-key");
    expect(out).toEqual({ ok: true, partial: { company: "MCo", title: "Eng" } });
    expect(fetchSpy).toHaveBeenCalledWith(
      "https://api.mistral.ai/v1/chat/completions",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          Authorization: "Bearer test-key",
        }),
      }),
    );
    fetchSpy.mockRestore();
  });

  it("calls Gemini API and maps response", async () => {
    const payload = {
      candidates: [{ content: { parts: [{ text: '{"company":"GCo"}' }] } }],
    };
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify(payload), { status: 200, headers: { "Content-Type": "application/json" } }),
    );
    const out = await extractJobInfo("text", "gemini", "g-key");
    expect(out).toEqual({ ok: true, partial: { company: "GCo" } });
    fetchSpy.mockRestore();
  });

  it("returns API error text when Mistral HTTP fails", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockResolvedValue(
      new Response(JSON.stringify({ message: "Invalid API key" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      }),
    );
    const out = await extractJobInfo("text", "mistral", "bad");
    expect(out).toEqual({ ok: false, error: "HTTP 401: Invalid API key" });
    fetchSpy.mockRestore();
  });
});
