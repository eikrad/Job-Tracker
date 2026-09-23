// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { en } from "../../i18n/en";
import { MailMatchInboxPanel } from "./MailMatchInboxPanel";
import type { AcceptOutcome, DismissedRow, RunRow } from "./mailMatchApi";
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
    snippetOnly: false,
    pass1Score: null,
    pass1Reason: null,
    pass2Score: null,
    pass2Reason: null,
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
    dismiss: vi.fn<(inboxId: number) => Promise<void>>(async () => {}),
    restore: vi.fn(async () => {}),
    acceptNew: vi.fn<(inboxId: number) => Promise<AcceptOutcome>>(async () => ({
      jobId: 42,
      fieldsWritten: ["title"],
      created: true,
    })),
    undoAccept: vi.fn(async () => {}),
    openUrl: vi.fn(async () => {}),
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

  it("says a listing is mail-snippet-only rather than incomplete when its board blocks fetching", async () => {
    renderPanel(
      makeApi({
        rows: [
          row({
            enrichmentState: "skipped",
            enrichmentError: "listing page not fetchable",
            snippetOnly: true,
          }),
        ],
      }),
    );
    expect(await screen.findByText(t.badgeSnippetOnly)).toBeTruthy();
    expect(screen.queryByText(t.badgeIncompleteEnrichment)).toBeNull();
  });

  it("still calls a skipped fetch incomplete when it was not the board's doing", async () => {
    renderPanel(makeApi({ rows: [row({ enrichmentState: "skipped", snippetOnly: false })] }));
    expect(await screen.findByText(t.badgeIncompleteEnrichment)).toBeTruthy();
    expect(screen.queryByText(t.badgeSnippetOnly)).toBeNull();
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

describe("reading a match", () => {
  const fullAd = "Acme builds trains.\n\nYou will write Rust for signalling systems.";

  function adRow(overrides: Partial<MailMatchRow> = {}) {
    return row({
      pass1Score: 7,
      pass1Reason: "Rust, Copenhagen",
      pass2Score: 9,
      pass2Reason: "fits the full profile",
      draftJson: JSON.stringify({
        title: "Rust Engineer",
        raw_text: fullAd,
        url: "https://careers.acme.example/ad/7",
        board_url: "https://www.jobindex.dk/c?t=h7",
      }),
      ...overrides,
    });
  }

  it("shows the full ad and both score reasons when a row is opened", async () => {
    renderPanel(makeApi({ rows: [adRow()] }));

    fireEvent.click(await screen.findByRole("button", { expanded: false }));

    const detail = await screen.findByRole("region", { name: t.detailRegion("Rust Engineer") });
    // The whole ad, line breaks intact — not the one-line teaser from the mail.
    expect(within(detail).getByText(/You will write Rust/).textContent).toBe(fullAd);
    expect(within(detail).getByText(t.passScore(1, "7"))).toBeTruthy();
    expect(within(detail).getByText("Rust, Copenhagen")).toBeTruthy();
    expect(within(detail).getByText(t.passScore(2, "9"))).toBeTruthy();
    expect(within(detail).getByText("fits the full profile")).toBeTruthy();
  });

  it("says when a pass gave no reason", async () => {
    renderPanel(makeApi({ rows: [adRow({ pass2Score: null, pass2Reason: null })] }));

    fireEvent.click(await screen.findByRole("button", { expanded: false }));

    expect(await screen.findByText(t.passNotRun(2))).toBeTruthy();
  });

  it("opens the ad and its board link in the browser", async () => {
    const api = makeApi({ rows: [adRow()] });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { expanded: false }));
    fireEvent.click(await screen.findByRole("link", { name: t.openListing }));
    fireEvent.click(screen.getByRole("link", { name: t.viaBoard("jobindex.dk") }));

    expect(api.openUrl).toHaveBeenNthCalledWith(1, "https://careers.acme.example/ad/7");
    expect(api.openUrl).toHaveBeenNthCalledWith(2, "https://www.jobindex.dk/c?t=h7");
  });

  it("falls back to the listing link when the draft has none", async () => {
    const api = makeApi({
      rows: [row({ draftJson: JSON.stringify({ raw_text: "x" }), listingUrl: "https://example.com/job" })],
    });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { expanded: false }));
    fireEvent.click(await screen.findByRole("link", { name: t.openListing }));

    expect(api.openUrl).toHaveBeenCalledWith("https://example.com/job");
    expect(screen.queryByText(/^via /)).toBeNull();
  });
});

describe("keyboard triage", () => {
  const twoRows = () => [
    row({ id: 1, fingerprintId: "fp-1", title: "Rust Engineer" }),
    row({ id: 2, fingerprintId: "fp-2", title: "Platform Engineer" }),
  ];

  function press(key: string, target: Element = document.activeElement ?? document.body) {
    fireEvent.keyDown(target, { key });
  }

  /** Title of the row keyboard focus is on, or null. */
  function currentTitle(): string | null {
    const current = screen
      .queryAllByRole("listitem")
      .find((li) => li.getAttribute("aria-current") === "true");
    return current?.querySelector("strong")?.textContent ?? null;
  }

  it("moves a visible focus through the rows with j/k and the arrow keys", async () => {
    renderPanel(makeApi({ rows: twoRows() }));
    await screen.findByText("Platform Engineer");

    press("j");
    expect(currentTitle()).toBe("Rust Engineer");
    press("j");
    expect(currentTitle()).toBe("Platform Engineer");
    press("k");
    expect(currentTitle()).toBe("Rust Engineer");
    press("ArrowDown");
    expect(currentTitle()).toBe("Platform Engineer");
    press("ArrowUp");
    expect(currentTitle()).toBe("Rust Engineer");
    // The focused row really has keyboard focus, so screen readers follow along.
    expect(document.activeElement?.textContent).toContain("Rust Engineer");
  });

  it("opens and closes the focused row's detail with Enter or o", async () => {
    renderPanel(makeApi({ rows: twoRows() }));
    await screen.findByText("Platform Engineer");

    press("j");
    press("Enter");
    expect(await screen.findByRole("region", { name: t.detailRegion("Rust Engineer") })).toBeTruthy();
    press("o");
    expect(screen.queryByRole("region", { name: t.detailRegion("Rust Engineer") })).toBeNull();
  });

  it("accepts the focused row with a and moves on to the next one", async () => {
    const api = makeApi({ rows: twoRows() });
    renderPanel(api);
    await screen.findByText("Platform Engineer");

    press("j");
    press("a");

    await waitFor(() => expect(api.acceptNew).toHaveBeenCalledWith(1));
    await waitFor(() => expect(currentTitle()).toBe("Platform Engineer"));
  });

  it("dismisses the focused row with d and moves on to the next one", async () => {
    const api = makeApi({ rows: twoRows() });
    renderPanel(api);
    await screen.findByText("Platform Engineer");

    press("j");
    press("d");

    await waitFor(() => expect(api.dismiss).toHaveBeenCalledWith(1));
    await waitFor(() => expect(currentTitle()).toBe("Platform Engineer"));
  });

  it("undoes the last accept with u", async () => {
    const api = makeApi({ rows: twoRows() });
    renderPanel(api);
    await screen.findByText("Platform Engineer");

    press("j");
    press("a");
    await screen.findByText(t.acceptedNotice("Rust Engineer"));
    press("u");

    await waitFor(() => expect(api.undoAccept).toHaveBeenCalledWith(1));
  });

  it("undoes the last dismiss with u", async () => {
    const api = makeApi({ rows: twoRows() });
    renderPanel(api);
    await screen.findByText("Platform Engineer");

    press("j");
    press("d");
    await waitFor(() => expect(currentTitle()).toBe("Platform Engineer"));
    press("u");

    await waitFor(() => expect(api.restore).toHaveBeenCalledWith("fp-1"));
  });

  it("ignores shortcuts while typing in the search box", async () => {
    const api = makeApi({ rows: twoRows() });
    renderPanel(api);
    await screen.findByText("Platform Engineer");
    press("j");

    const search = screen.getByRole("searchbox");
    search.focus();
    press("a", search);
    press("d", search);
    press("j", search);

    expect(api.acceptNew).not.toHaveBeenCalled();
    expect(api.dismiss).not.toHaveBeenCalled();
    expect(currentTitle()).toBe("Rust Engineer");
  });

  it("leaves keys alone while a dialog is open", async () => {
    // The scan sheet sits on the same page; its buttons must not accept matches.
    const api = makeApi({ rows: twoRows() });
    renderPanel(api);
    await screen.findByText("Platform Engineer");
    press("j");

    const dialog = document.createElement("div");
    dialog.setAttribute("role", "dialog");
    const button = document.createElement("button");
    dialog.append(button);
    document.body.append(dialog);
    press("a", button);
    dialog.remove();

    expect(api.acceptNew).not.toHaveBeenCalled();
  });

  it("shows which keys do what", async () => {
    renderPanel(makeApi({ rows: twoRows() }));
    expect(await screen.findByText(t.shortcutsHint)).toBeTruthy();
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

  it("offers Undo right after a dismiss, which restores the match", async () => {
    const api = makeApi({ rows: [row()] });
    renderPanel(api);

    fireEvent.click(await screen.findByRole("button", { name: t.dismiss }));
    expect(await screen.findByText(t.dismissedNotice("Rust Engineer"))).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: t.acceptedUndo }));

    await waitFor(() => expect(api.restore).toHaveBeenCalledWith("fp-1"));
    expect(await screen.findByText(t.dismissUndoneNotice)).toBeTruthy();
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

describe("bulk actions", () => {
  const threeRows = () => [
    row({ id: 1, fingerprintId: "fp-1", title: "Rust Engineer" }),
    row({ id: 2, fingerprintId: "fp-2", title: "Platform Engineer" }),
    row({ id: 3, fingerprintId: "fp-3", title: "Barista" }),
  ];
  // happy-dom has no window.confirm, so the confirmation is stubbed in.
  const confirm = vi.fn<(message?: string) => boolean>(() => true);

  beforeEach(() => {
    confirm.mockReset().mockReturnValue(true);
    vi.stubGlobal("confirm", confirm);
  });
  afterEach(() => vi.unstubAllGlobals());

  async function select(...titles: string[]) {
    for (const title of titles) {
      fireEvent.click(await screen.findByRole("checkbox", { name: t.selectRow(title) }));
    }
  }

  it("accepts the selected matches after confirming the count", async () => {
    const api = makeApi({ rows: threeRows() });
    const { onJobsChanged } = renderPanel(api);
    await select("Rust Engineer", "Platform Engineer");

    fireEvent.click(screen.getByRole("button", { name: t.bulkAccept(2) }));

    expect(confirm).toHaveBeenCalledWith(t.bulkAcceptConfirm(2));
    await screen.findByText(t.bulkAcceptedNotice(2, 2));
    expect(api.acceptNew.mock.calls).toEqual([[1], [2]]);
    // One reload for the whole batch, not one per match.
    expect(onJobsChanged).toHaveBeenCalledTimes(1);
    expect(api.list).toHaveBeenCalledTimes(2);
  });

  it("dismisses the selected matches after confirming the count", async () => {
    const api = makeApi({ rows: threeRows() });
    renderPanel(api);
    await select("Rust Engineer", "Barista");

    fireEvent.click(screen.getByRole("button", { name: t.bulkDismiss(2) }));

    expect(confirm).toHaveBeenCalledWith(t.bulkDismissConfirm(2));
    await screen.findByText(t.bulkDismissedNotice(2, 2));
    expect(api.dismiss.mock.calls).toEqual([[1], [3]]);
  });

  it("does nothing when the confirmation is declined", async () => {
    confirm.mockReturnValue(false);
    const api = makeApi({ rows: threeRows() });
    renderPanel(api);
    await select("Rust Engineer");

    fireEvent.click(screen.getByRole("button", { name: t.bulkAccept(1) }));

    expect(api.acceptNew).not.toHaveBeenCalled();
  });

  it("runs one match at a time", async () => {
    const api = makeApi({ rows: threeRows() });
    let inFlight = 0;
    let most = 0;
    api.dismiss.mockImplementation(async () => {
      inFlight += 1;
      most = Math.max(most, inFlight);
      await new Promise((r) => setTimeout(r, 5));
      inFlight -= 1;
    });
    renderPanel(api);
    fireEvent.click(await screen.findByRole("checkbox", { name: t.selectAllShown }));

    fireEvent.click(screen.getByRole("button", { name: t.bulkDismiss(3) }));

    await screen.findByText(t.bulkDismissedNotice(3, 3));
    expect(most).toBe(1);
  });

  it("reports which matches failed and keeps going", async () => {
    const api = makeApi({ rows: threeRows() });
    api.acceptNew.mockImplementation(async (id: number) => {
      if (id === 2) throw new Error("database is locked");
      return { jobId: 40 + id, fieldsWritten: [], created: true };
    });
    const { onJobsChanged } = renderPanel(api);
    fireEvent.click(await screen.findByRole("checkbox", { name: t.selectAllShown }));

    fireEvent.click(screen.getByRole("button", { name: t.bulkAccept(3) }));

    expect(await screen.findByText(t.bulkAcceptedNotice(2, 3))).toBeTruthy();
    expect(screen.getByText(t.bulkFailure("Platform Engineer", "Error: database is locked"))).toBeTruthy();
    expect(api.acceptNew.mock.calls).toEqual([[1], [2], [3]]);
    expect(onJobsChanged).toHaveBeenCalledTimes(1);
  });

  it("only acts on selected rows the filters still show", async () => {
    const api = makeApi({ rows: threeRows() });
    renderPanel(api);
    await select("Rust Engineer", "Barista");

    fireEvent.change(screen.getByRole("searchbox"), { target: { value: "Rust" } });

    expect(screen.getByRole("button", { name: t.bulkDismiss(1) })).toBeTruthy();
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
