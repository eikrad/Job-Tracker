/**
 * The error codes a scan run can end with (`mail_scan_runs.error_code`, written by
 * `src-tauri/src/mail_scan/mod.rs`) to one user-facing sentence each (spec §10).
 *
 * Only run codes belong here. Problems caught before a run starts — a missing key or
 * profile — come back from `mail_scan_start` as a message the caller shows directly.
 * Its own module so the card stays a component-only file.
 */

import { en } from "../../i18n/en";

const t = en.mailMatch;

export function errorMessage(code: string | null | undefined): string | null {
  if (!code) return null;
  // The sidecar exited with a status (`E_EXIT_<n>`), died without one, or its output
  // stream broke: the same story for the user either way.
  if (code.startsWith("E_EXIT_")) return t.errorSidecarStopped;
  switch (code) {
    case "E_SPAWN":
      return t.errorSidecarMissing;
    case "E_PROTOCOL_MISMATCH":
      return t.errorSidecarVersion;
    case "E_PROTOCOL":
    case "E_PROTOCOL_OVERSIZE":
      return t.errorProtocol;
    case "E_IO":
    case "E_CHILD":
      return t.errorSidecarStopped;
    case "E_LLM_AUTH":
      return t.errorLlmAuth;
    case "E_LLM_MODEL":
      return t.errorLlmModel;
    case "E_LLM_UNAVAILABLE":
      return t.errorLlmUnavailable;
    case "E_LLM":
      return t.errorLlmOther;
    case "E_DB":
      return t.errorDb;
    default:
      return t.errorUnknown;
  }
}
