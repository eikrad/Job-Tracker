// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { en } from "../../i18n/en";
import { MailMatchInboxPanel } from "./MailMatchInboxPanel";
import type { DismissedRow, RunRow, UpdatePreview } from "./mailMatchApi";
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
    kind: "new",
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
  preview?: UpdatePreview;
} = {}) {
  return {
    list: vi.fn(async () => overrides.rows ?? []),
    listDismissed: vi.fn(async () => overrides.dismissed ?? []),
    listRuns: vi.fn(async () => overrides.runs ?? []),
    dismiss: vi.fn(async () => {}),
    restore: vi.fn(async () => {}),
    previewUpdate: vi.fn(
      async () =>
        overrides.preview ?? {
          inboxId: 1,
          jobId: 7,
          jobChangedSinceScan: false,
          fields: [],
        },
    ),
    acceptUpdate: vi.fn(async () => ({ jobId: 7, fieldsWritten: [], created: true })),
  };
}

function renderPanel(
  api: ReturnType<typeof makeApi>,
  onAcceptDraft = vi.fn(),
) {
  render(<MailMatchInboxPanel api={api} onAcceptDraft={onAcceptDraft} />);
  return { onAcceptDraft };
}

describe("rendering", () => {
  it("lists pending matches highest score first", async () => {
    const api = makeApi({
      rows: [
        row({ id: 1, score: 4, title: "Barista" }),
        row({ id: 2, fingerprintId: "fp-2", score: 9, title: "Senior Rust Engineer" }),
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
  it("opens the prefilled form instead of writing silently", async () => {
    // Spec non-goal 4: no path from a scan to a written Job without a human.
    const api = makeApi({ rows: [row()] });
    const onAcceptDraft = vi.fn();
    renderPanel(api, onAcceptDraft);

    fireEvent.click(await screen.findByRole("button", { name: t.accept }));

    expect(onAcceptDraft).toHaveBeenCalledTimes(1);
    const [inboxId, draft] = onAcceptDraft.mock.calls[0];
    expect(inboxId).toBe(1);
    expect(draft).toMatchObject({ company: "Acme", title: "Rust Engineer" });
  });

  it("shows the recomputed diff for an update suggestion", async () => {
    const api = makeApi({
      rows: [row({ kind: "update_suggestion", jobId: 7 })],
      preview: {
        inboxId: 1,
        jobId: 7,
        jobChangedSinceScan: false,
        fields: [
          { field: "deadline", suggested: "2026-10-01", current: null, applicable: true },
        ],
      },
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.acceptUpdate }));

    expect(await screen.findByText(t.diffWillWrite)).toBeTruthy();
    expect(screen.getByText("2026-10-01")).toBeTruthy();
  });

  it("warns when the job changed since the scan", async () => {
    // The C2 property, surfaced: accepting must not look routine when the user's
    // own edit is at stake.
    const api = makeApi({
      rows: [row({ kind: "update_suggestion", jobId: 7 })],
      preview: {
        inboxId: 1,
        jobId: 7,
        jobChangedSinceScan: true,
        fields: [
          { field: "deadline", suggested: "2026-10-01", current: "2026-12-24", applicable: false },
          { field: "work_mode", suggested: "Hybrid", current: null, applicable: true },
        ],
      },
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.acceptUpdate }));

    expect(await screen.findByText(t.diffJobChanged)).toBeTruthy();
    expect(screen.getByText(t.diffApply(1))).toBeTruthy();
  });

  it("offers nothing to apply when the user already filled everything in", async () => {
    const api = makeApi({
      rows: [row({ kind: "update_suggestion", jobId: 7 })],
      preview: {
        inboxId: 1,
        jobId: 7,
        jobChangedSinceScan: true,
        fields: [
          { field: "deadline", suggested: "2026-10-01", current: "2026-12-24", applicable: false },
        ],
      },
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.acceptUpdate }));

    expect(await screen.findByText(t.diffNothingToDo)).toBeTruthy();
    expect(screen.getByRole("button", { name: t.diffApply(0) }).hasAttribute("disabled")).toBe(true);
  });
});

describe("dismiss and restore", () => {
  it("dismisses with an optional reason", async () => {
    const api = makeApi({ rows: [row()] });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.dismiss }));
    fireEvent.change(screen.getByLabelText(t.dismissReasonLabel), {
      target: { value: "recruiter spam" },
    });
    fireEvent.click(screen.getByRole("button", { name: t.dismissConfirm }));

    await waitFor(() => expect(api.dismiss).toHaveBeenCalledWith(1, "recruiter spam"));
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
          statsJson: '{"inboxNew":11,"updates":3,"underCutoff":52,"suppressedByDismissal":7}',
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
    const { container } = render(
      <MailMatchInboxPanel api={api} onAcceptDraft={vi.fn()} />,
    );

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
