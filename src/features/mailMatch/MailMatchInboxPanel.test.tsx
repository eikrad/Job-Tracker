// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { en } from "../../i18n/en";
import { MailMatchInboxPanel } from "./MailMatchInboxPanel";
import type { DismissedRow, RunRow } from "./mailMatchApi";
import type { MailMatchRow } from "./mailMatchInbox";
// `?raw` rather than node:fs — the app tsconfig ships no node types, and this keeps
// the guard inside the same module graph as the code it guards.
import panelSource from "./MailMatchInboxPanel.tsx?raw";
import runSummaryCardSource from "./RunSummaryCard.tsx?raw";
import scanControlSource from "./ScanControl.tsx?raw";
import inboxLogicSource from "./mailMatchInbox.ts?raw";

const t = en.mailMatch;

// vitest runs without globals, so RTL's automatic cleanup never registers.
afterEach(cleanup);

function row(overrides: Partial<MailMatchRow> = {}): MailMatchRow {
  return {
    id: 1,
    fingerprintId: "fp-1",
    status: "pending",
    jobId: null,
    score: 8,
    scoreReason: "strong Rust match",
    scoreState: "ok",
    suspicious: false,
    nearDuplicateOf: null,
    enrichmentState: "complete",
    enrichmentError: null,
    draftJson: JSON.stringify({
      title: "Rust Engineer",
      company: "Acme",
      raw_text: "We are hiring a Rust engineer.",
    }),
    sourceBoard: "indeed",
    messageDate: "2026-09-08T06:12:00Z",
    listingUrl: "https://example.com/job",
    title: "Rust Engineer",
    company: "Acme",
    seenCount: 1,
    lastSeenAt: "2026-09-08T06:12:00Z",
    updatedAt: "2026-09-08T06:12:00Z",
    ...overrides,
  };
}

function makeApi(overrides: {
  rows?: MailMatchRow[];
  dismissed?: DismissedRow[];
  runs?: RunRow[];
} = {}) {
  return {
    list: vi.fn(async () => overrides.rows ?? []),
    listDismissed: vi.fn(async () => overrides.dismissed ?? []),
    listRuns: vi.fn(async () => overrides.runs ?? []),
    dismiss: vi.fn(async () => {}),
    restore: vi.fn(async () => {}),
    acceptNew: vi.fn(async () => ({ jobId: 42, fieldsWritten: ["title"], created: true })),
    undoAccept: vi.fn(async () => {}),
  };
}

function renderPanel(api: ReturnType<typeof makeApi>) {
  const onJobsChanged = vi.fn();
  const onOpenJob = vi.fn();
  render(<MailMatchInboxPanel api={api} onJobsChanged={onJobsChanged} onOpenJob={onOpenJob} />);
  return { onJobsChanged, onOpenJob };
}

describe("rendering", () => {
  it("lists pending matches in the backend's ranking", async () => {
    const api = makeApi({
      rows: [
        row({ id: 2, fingerprintId: "fp-2", score: 9, title: "Senior Rust Engineer" }),
        row({ id: 1, score: 4, title: "Barista" }),
      ],
    });
    renderPanel(api);

    const items = await screen.findAllByRole("listitem");
    expect(within(items[0]).getByText("Senior Rust Engineer")).toBeTruthy();
  });

  it("shows the empty state when nothing is waiting", async () => {
    renderPanel(makeApi({ rows: [] }));
    expect(await screen.findByText(t.emptyPending)).toBeTruthy();
  });

  it("renders an invalid score as ? rather than a number", async () => {
    renderPanel(makeApi({ rows: [row({ score: null, scoreState: "invalid" })] }));
    const chip = await screen.findByLabelText(t.scoreAria("?"));
    expect(chip.textContent).toBe("?");
  });

  it("shows badges for incomplete enrichment, suspicion, and repeat sightings", async () => {
    renderPanel(
      makeApi({
        rows: [row({ enrichmentState: "partial", suspicious: true, seenCount: 3 })],
      }),
    );
    expect(await screen.findByText(t.badgeIncompleteEnrichment)).toBeTruthy();
    expect(screen.getByText(t.badgeSuspicious)).toBeTruthy();
    expect(screen.getByText(t.seenTimes(3))).toBeTruthy();
  });

  it("shows near-duplicates as a pair rather than merging them", async () => {
    const api = makeApi({
      rows: [
        row({ id: 1, fingerprintId: "fp-kbh", nearDuplicateOf: "fp-aarhus" }),
        row({ id: 2, fingerprintId: "fp-aarhus", title: "Rust Engineer (Aarhus)" }),
      ],
    });
    renderPanel(api);

    const items = await screen.findAllByRole("listitem");
    expect(items.length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText(t.nearDuplicateHint)).toBeTruthy();
  });
});

describe("accept", () => {
  it("creates the job in one click and offers to open it", async () => {
    const api = makeApi({ rows: [row()] });
    const { onJobsChanged, onOpenJob } = renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.accept }));

    await waitFor(() => expect(api.acceptNew).toHaveBeenCalledWith(1));
    expect(await screen.findByText(t.acceptedNotice("Rust Engineer"))).toBeTruthy();
    expect(onJobsChanged).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: t.acceptedOpen }));
    expect(onOpenJob).toHaveBeenCalledWith(42);
  });

  it("undoes an accept from the notice", async () => {
    // Undo replaces the old prefilled form as the human check on a one-click accept.
    const api = makeApi({ rows: [row()] });
    const { onJobsChanged } = renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.accept }));
    fireEvent.click(await screen.findByRole("button", { name: t.acceptedUndo }));

    await waitFor(() => expect(api.undoAccept).toHaveBeenCalledWith(1));
    expect(await screen.findByText(t.undoneNotice)).toBeTruthy();
    expect(onJobsChanged).toHaveBeenCalledTimes(2);
  });

});

describe("dismiss and restore", () => {
  it("dismisses in one click without asking for a reason", async () => {
    const api = makeApi({ rows: [row()] });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.dismiss }));

    await waitFor(() => expect(api.dismiss).toHaveBeenCalledWith(1));
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("lists dismissals and restores them", async () => {
    // A dismissal the user cannot see or undo is the failure this tab prevents.
    const api = makeApi({
      dismissed: [
        {
          fingerprintId: "fp-gone",
          reason: "recruiter spam",
          dismissedAt: "2026-09-09T00:00:00Z",
          dismissedRun: "r1",
          title: "Barista",
          company: "Cafe",
          listingUrl: null,
        },
      ],
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("tab", { name: /Dismissed/ }));
    expect(await screen.findByText("Barista")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: t.restore }));
    await waitFor(() => expect(api.restore).toHaveBeenCalledWith("fp-gone"));
  });

  it("shows an empty state when nothing is dismissed", async () => {
    renderPanel(makeApi({ dismissed: [] }));
    fireEvent.click(await screen.findByRole("tab", { name: /Dismissed/ }));
    expect(await screen.findByText(t.emptyDismissed)).toBeTruthy();
  });
});

describe("history", () => {
  it("renders a finished run from its stats_json", async () => {
    const api = makeApi({
      runs: [
        {
          runId: "ms_1",
          status: "completed",
          startedAt: "2026-09-09T10:00:00Z",
          finishedAt: "2026-09-09T10:04:00Z",
          statsJson: '{"inboxNew":11,"alreadyTracked":3,"underCutoff":52,"suppressedByDismissal":7}',
          errorCode: null,
          errorSummary: null,
          modelId: "deepseek-v4-flash-0731",
        },
      ],
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("tab", { name: /History/ }));

    expect(await screen.findByText(t.summaryNew(11))).toBeTruthy();
    expect(screen.getByText(t.summaryUnderCutoff(52))).toBeTruthy();
    expect(screen.getByText(t.summaryAlreadyTracked(3))).toBeTruthy();
    expect(screen.getByText(t.statusCompleted)).toBeTruthy();
  });

  it("links a suppressed count to the Dismissed tab", async () => {
    const api = makeApi({
      runs: [
        {
          runId: "ms_1",
          status: "completed",
          startedAt: "t",
          finishedAt: "t",
          statsJson: '{"suppressedByDismissal":7}',
          errorCode: null,
          errorSummary: null,
          modelId: null,
        },
      ],
      dismissed: [],
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("tab", { name: /History/ }));
    fireEvent.click(await screen.findByRole("button", { name: t.summarySuppressedLink }));

    expect(await screen.findByText(t.emptyDismissed)).toBeTruthy();
  });

  it("explains a failed run in one sentence", async () => {
    const api = makeApi({
      runs: [
        {
          runId: "ms_1",
          status: "failed",
          startedAt: "t",
          finishedAt: "t",
          statsJson: "{}",
          errorCode: "E_LLM_UNAVAILABLE",
          errorSummary: "redacted detail",
          modelId: null,
        },
      ],
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("tab", { name: /History/ }));
    expect(await screen.findByText(t.errorLlmUnavailable)).toBeTruthy();
  });
});

describe("untrusted content", () => {
  it("renders listing text as text, not markup", async () => {
    const hostile = '<img src=x onerror="alert(1)"><b>bold</b>';
    const api = makeApi({
      rows: [row({ draftJson: JSON.stringify({ raw_text: hostile }) })],
    });
    const { container } = render(<MailMatchInboxPanel api={api} />);

    fireEvent.click(await screen.findByRole("button", { expanded: false }));

    await waitFor(() => expect(screen.getByText(hostile)).toBeTruthy());
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("b")).toBeNull();
  });

  it("has no dangerouslySetInnerHTML anywhere on this path", () => {
    // The webview holds no API keys after PR A, but it does hold the user's data,
    // and everything rendered here originated in an email.
    //
    // Comments are stripped first: prose that *names* the attribute (including the
    // one explaining this rule) must not trip the guard, and must not be a way to
    // satisfy it either.
    const stripComments = (src: string) =>
      src.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/[^\n]*/g, "");

    const sources: [string, string][] = [
      ["MailMatchInboxPanel.tsx", panelSource],
      ["RunSummaryCard.tsx", runSummaryCardSource],
      ["ScanControl.tsx", scanControlSource],
      ["mailMatchInbox.ts", inboxLogicSource],
    ];
    for (const [file, source] of sources) {
      expect(stripComments(source), `${file} must not render untrusted mail as HTML`).not.toContain(
        "dangerouslySetInnerHTML",
      );
    }
  });
});
