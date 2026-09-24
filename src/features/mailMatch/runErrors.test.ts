import { describe, expect, it } from "vitest";
import { en } from "../../i18n/en";
import { errorMessage } from "./runErrors";

const t = en.mailMatch;

describe("run error messages", () => {
  // Every code `src-tauri/src/mail_scan/mod.rs` can write to `mail_scan_runs.error_code`.
  // A code that falls through to "The scan failed." tells the user nothing.
  it.each([
    "E_SPAWN",
    "E_PROTOCOL_MISMATCH",
    "E_PROTOCOL",
    "E_PROTOCOL_OVERSIZE",
    "E_IO",
    "E_CHILD",
    "E_EXIT_1",
    "E_DB",
    "E_LLM_AUTH",
    "E_LLM_MODEL",
    "E_LLM_UNAVAILABLE",
    "E_LLM",
    "E_INTERRUPTED",
  ])("explains %s", (code) => {
    const message = errorMessage(code);
    expect(message).toBeTruthy();
    expect(message).not.toBe(t.errorUnknown);
  });

  it("treats any sidecar exit status as the scanner stopping", () => {
    expect(errorMessage("E_EXIT_2")).toBe(errorMessage("E_CHILD"));
    expect(errorMessage("E_EXIT_137")).toBe(t.errorSidecarStopped);
  });

  it("has nothing to say about a run without an error", () => {
    expect(errorMessage(null)).toBeNull();
    expect(errorMessage(undefined)).toBeNull();
  });

  it("falls back to a generic sentence for a code it does not know", () => {
    expect(errorMessage("E_SOMETHING_NEW")).toBe(t.errorUnknown);
  });
});
