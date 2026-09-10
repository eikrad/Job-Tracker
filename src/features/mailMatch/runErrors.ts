/**
 * Stable error codes to one user-facing sentence each (spec §10).
 *
 * Its own module so the card stays a component-only file, and so the mapping can be
 * tested without rendering anything.
 */

import { en } from "../../i18n/en";

const t = en.mailMatch;

export function errorMessage(code: string | null | undefined): string | null {
  if (!code) return null;
  switch (code) {
    case "E_CONFIG_INCOMPLETE":
      return t.errorConfigIncomplete;
    case "E_PROFILE_UNREADABLE":
      return t.errorProfileUnreadable;
    case "E_SOURCE_UNREADABLE":
      return t.errorSourceUnreadable;
    case "E_SIDECAR_MISSING":
    case "E_SPAWN":
      return t.errorSidecarMissing;
    case "E_SIDECAR_VERSION":
    case "E_PROTOCOL_MISMATCH":
      return t.errorSidecarVersion;
    case "E_PROTOCOL":
    case "E_PROTOCOL_OVERSIZE":
      return t.errorProtocol;
    case "E_LLM_AUTH":
      return t.errorLlmAuth;
    case "E_LLM_MODEL":
      return t.errorLlmModel;
    case "E_LLM_UNAVAILABLE":
      return t.errorLlmUnavailable;
    case "E_BUDGET_EXHAUSTED":
      return t.errorBudgetExhausted;
    case "E_DB":
      return t.errorDb;
    default:
      return t.errorUnknown;
  }
}

