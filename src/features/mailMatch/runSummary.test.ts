import { describe, expect, it } from "vitest";
import {
  applyProgress,
  emptyStats,
  isTerminal,
  parseStats,
  progressPercent,
  runViewFromRow,
  type RunView,
} from "./runSummary";

describe("parseStats", () => {
  it("reads a stats_json string from a finished run", () => {
    const stats = parseStats('{"inboxNew":11,"underCutoff":52,"suppressedByDismissal":7}');
    expect(stats.inboxNew).toBe(11);
    expect(stats.underCutoff).toBe(52);
    expect(stats.suppressedByDismissal).toBe(7);
  });

  it("treats a run that crashed before writing stats as all zeroes", () => {
    expect(parseStats("{}")).toEqual(emptyStats);
    expect(parseStats("not json")).toEqual(emptyStats);
    expect(parseStats(null)).toEqual(emptyStats);
  });

  it("defaults a counter an older row predates", () => {
    // A row written before `enrichmentFailures` existed must render, not throw.
    const stats = parseStats('{"inboxNew":3}');
    expect(stats.enrichmentFailures).toBe(0);
    expect(stats.inboxNew).toBe(3);
  });

  it("ignores values of the wrong type rather than rendering NaN", () => {
    const stats = parseStats('{"inboxNew":"eleven","llmCalls":null}');
    expect(stats.inboxNew).toBe(0);
    expect(stats.llmCalls).toBe(0);
  });

  it("reads budgetExhausted as a strict boolean", () => {
    expect(parseStats('{"budgetExhausted":true}').budgetExhausted).toBe(true);
    expect(parseStats('{"budgetExhausted":"yes"}').budgetExhausted).toBe(false);
    expect(parseStats("{}").budgetExhausted).toBe(false);
  });
});

describe("live progress", () => {
  it("starts a view from the first event", () => {
    const view = applyProgress(null, {
      runId: "ms_1",
      listingsCommitted: 3,
      messagesSeen: 10,
      status: "running",
    });
    expect(view.runId).toBe("ms_1");
    expect(view.status).toBe("running");
    expect(view.stats.listingsCommitted).toBe(3);
  });

  it("never moves counters backwards", () => {
    // Events are coalesced at 4/s and can arrive out of order; a progress bar
    // that jumps backwards reads as a bug.
    let view = applyProgress(null, {
      runId: "ms_1",
      listingsCommitted: 10,
      messagesSeen: 40,
      status: "running",
    });
    view = applyProgress(view, {
      runId: "ms_1",
      listingsCommitted: 4,
      messagesSeen: 20,
      status: "running",
    });
    expect(view.stats.listingsCommitted).toBe(10);
    expect(view.stats.messagesSeen).toBe(40);
  });

  it("resets when a new run starts", () => {
    const first = applyProgress(null, {
      runId: "ms_1",
      listingsCommitted: 10,
      messagesSeen: 40,
      status: "running",
    });
    const second = applyProgress(first, {
      runId: "ms_2",
      listingsCommitted: 1,
      messagesSeen: 2,
      status: "running",
    });
    expect(second.runId).toBe("ms_2");
    expect(second.stats.listingsCommitted).toBe(1);
  });

  it("carries the terminal status through", () => {
    for (const status of ["completed", "cancelled", "failed"] as const) {
      const view = applyProgress(null, {
        runId: "ms_1",
        listingsCommitted: 1,
        messagesSeen: 1,
        status,
      });
      expect(view.status).toBe(status);
      expect(isTerminal(view)).toBe(true);
    }
  });

  it("treats an unknown status as still running", () => {
    const view = applyProgress(null, {
      runId: "ms_1",
      listingsCommitted: 0,
      messagesSeen: 0,
      status: "something-new",
    });
    expect(view.status).toBe("running");
    expect(isTerminal(view)).toBe(false);
  });
});

describe("history renders through the same reducer", () => {
  it("builds an equivalent view from a finished run row", () => {
    // The property that keeps History honest: a finished run and a live one
    // produce the same shape, so one component renders both.
    const view = runViewFromRow({
      runId: "ms_9",
      status: "completed",
      startedAt: "2026-09-09T10:00:00Z",
      finishedAt: "2026-09-09T10:04:00Z",
      statsJson: '{"inboxNew":11,"updates":3,"underCutoff":52,"suppressedByDismissal":7}',
      errorCode: null,
      errorSummary: null,
      modelId: "deepseek-v4-flash-0731",
    });

    expect(view.status).toBe("completed");
    expect(view.stats.inboxNew).toBe(11);
    expect(view.stats.updates).toBe(3);
    expect(view.modelId).toBe("deepseek-v4-flash-0731");

    const live: RunView = {
      runId: "ms_9",
      status: "completed",
      stats: parseStats(
        '{"inboxNew":11,"updates":3,"underCutoff":52,"suppressedByDismissal":7}',
      ),
    };
    expect(view.stats).toEqual(live.stats);
  });

  it("surfaces a failed run's error code", () => {
    const view = runViewFromRow({
      runId: "ms_x",
      status: "failed",
      startedAt: "t",
      finishedAt: "t",
      statsJson: "{}",
      errorCode: "E_LLM_UNAVAILABLE",
      errorSummary: "5 consecutive provider failures",
      modelId: null,
    });
    expect(view.status).toBe("failed");
    expect(view.errorCode).toBe("E_LLM_UNAVAILABLE");
  });
});

describe("progressPercent", () => {
  it("is null before the total is known", () => {
    expect(progressPercent({ runId: "r", status: "running", stats: { ...emptyStats } })).toBeNull();
  });

  it("reports a percentage once messages are counted", () => {
    const view: RunView = {
      runId: "r",
      status: "running",
      stats: { ...emptyStats, messagesSeen: 200, messagesParsed: 50 },
    };
    expect(progressPercent(view)).toBe(25);
  });

  it("never exceeds 100", () => {
    const view: RunView = {
      runId: "r",
      status: "running",
      stats: { ...emptyStats, messagesSeen: 10, messagesParsed: 50 },
    };
    expect(progressPercent(view)).toBe(100);
  });
});
