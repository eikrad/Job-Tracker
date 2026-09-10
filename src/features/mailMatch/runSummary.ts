/**
 * Run counters, shared by the live progress bar and the History view (spec §8.4).
 *
 * One renderer, two sources: a running scan feeds this reducer `mail-scan://progress`
 * events, and History feeds it `stats_json` off a finished run row. If the two ever
 * disagreed about what "52 below cutoff" means, History would quietly become fiction.
 */

export type RunStats = {
  listingsCommitted: number;
  messagesSeen: number;
  messagesParsed: number;
  suppressedByDismissal: number;
  underCutoff: number;
  inboxNew: number;
  updates: number;
  llmCalls: number;
  enrichmentFailures: number;
  errors: number;
  budgetExhausted: boolean;
};

export const emptyStats: RunStats = {
  listingsCommitted: 0,
  messagesSeen: 0,
  messagesParsed: 0,
  suppressedByDismissal: 0,
  underCutoff: 0,
  inboxNew: 0,
  updates: 0,
  llmCalls: 0,
  enrichmentFailures: 0,
  errors: 0,
  budgetExhausted: false,
};

export type RunStatus = "running" | "completed" | "cancelled" | "failed";

export type RunView = {
  runId: string;
  status: RunStatus;
  stats: RunStats;
  startedAt?: string;
  finishedAt?: string;
  errorCode?: string | null;
  errorSummary?: string | null;
  modelId?: string | null;
};

function asNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : 0;
}

/**
 * Coerce whatever a run row or a progress event carries into full counters.
 *
 * Tolerant on purpose: `stats_json` from a run that crashed early is `{}`, and an
 * older row may predate a counter. A missing counter is zero, not a broken view.
 */
export function parseStats(raw: unknown): RunStats {
  let source: Record<string, unknown> = {};
  if (typeof raw === "string") {
    try {
      const parsed: unknown = JSON.parse(raw);
      if (parsed && typeof parsed === "object") source = parsed as Record<string, unknown>;
    } catch {
      return { ...emptyStats };
    }
  } else if (raw && typeof raw === "object") {
    source = raw as Record<string, unknown>;
  }

  return {
    listingsCommitted: asNumber(source.listingsCommitted),
    messagesSeen: asNumber(source.messagesSeen),
    messagesParsed: asNumber(source.messagesParsed),
    suppressedByDismissal: asNumber(source.suppressedByDismissal),
    underCutoff: asNumber(source.underCutoff),
    inboxNew: asNumber(source.inboxNew),
    updates: asNumber(source.updates),
    llmCalls: asNumber(source.llmCalls),
    enrichmentFailures: asNumber(source.enrichmentFailures),
    errors: asNumber(source.errors),
    budgetExhausted: source.budgetExhausted === true,
  };
}

export type ProgressEvent = {
  runId: string;
  listingsCommitted: number;
  messagesSeen: number;
  status: string;
};

function asStatus(value: string): RunStatus {
  return value === "completed" || value === "cancelled" || value === "failed"
    ? value
    : "running";
}

/**
 * Fold one progress event into the current view.
 *
 * Counters only ever move forward within a run: events are coalesced at 4/s and can
 * arrive out of order, and a progress bar that jumps backwards reads as a bug.
 */
export function applyProgress(current: RunView | null, event: ProgressEvent): RunView {
  const base =
    current && current.runId === event.runId
      ? current
      : { runId: event.runId, status: "running" as RunStatus, stats: { ...emptyStats } };

  return {
    ...base,
    status: asStatus(event.status),
    stats: {
      ...base.stats,
      listingsCommitted: Math.max(base.stats.listingsCommitted, event.listingsCommitted),
      messagesSeen: Math.max(base.stats.messagesSeen, event.messagesSeen),
    },
  };
}

/** Build a view from a finished run row, for History. */
export function runViewFromRow(row: {
  runId: string;
  status: string;
  startedAt: string;
  finishedAt: string | null;
  statsJson: string;
  errorCode: string | null;
  errorSummary: string | null;
  modelId: string | null;
}): RunView {
  return {
    runId: row.runId,
    status: asStatus(row.status),
    stats: parseStats(row.statsJson),
    startedAt: row.startedAt,
    finishedAt: row.finishedAt ?? undefined,
    errorCode: row.errorCode,
    errorSummary: row.errorSummary,
    modelId: row.modelId,
  };
}

export function isTerminal(view: RunView): boolean {
  return view.status !== "running";
}

/** Percentage for the progress bar, or `null` when the total is not yet known. */
export function progressPercent(view: RunView): number | null {
  const total = view.stats.messagesSeen;
  if (total <= 0) return null;
  const done = Math.min(view.stats.messagesParsed || view.stats.listingsCommitted, total);
  return Math.round((done / total) * 100);
}
