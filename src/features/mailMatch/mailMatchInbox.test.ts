import { describe, expect, it } from "vitest";
import {
  badgesFor,
  boardOptions,
  defaultFilters,
  filterRows,
  listingText,
  pairNearDuplicates,
  parseDraft,
  scoreLabel,
  sortRows,
  visibleRows,
  type MailMatchRow,
} from "./mailMatchInbox";

function row(overrides: Partial<MailMatchRow> = {}): MailMatchRow {
  return {
    id: 1,
    fingerprintId: "fp-1",
    kind: "new",
    status: "pending",
    jobId: null,
    score: 8,
    scoreReason: "good match",
    scoreState: "ok",
    suspicious: false,
    nearDuplicateOf: null,
    enrichmentState: "complete",
    enrichmentError: null,
    draftJson: JSON.stringify({ title: "Rust Engineer", company: "Acme" }),
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

describe("ordering", () => {
  it("sorts by score, then by most recently seen", () => {
    const rows = [
      row({ id: 1, score: 4, lastSeenAt: "2026-09-09T00:00:00Z" }),
      row({ id: 2, score: 9, lastSeenAt: "2026-09-01T00:00:00Z" }),
      row({ id: 3, score: 9, lastSeenAt: "2026-09-09T00:00:00Z" }),
    ];
    expect(sortRows(rows).map((r) => r.id)).toEqual([3, 2, 1]);
  });

  it("puts an invalid score below every real score", () => {
    // A `?` outranking a 9 would push the rows we trust least to the top.
    const rows = [
      row({ id: 1, score: null, scoreState: "invalid", lastSeenAt: "2026-09-09T00:00:00Z" }),
      row({ id: 2, score: 2, lastSeenAt: "2026-09-01T00:00:00Z" }),
    ];
    expect(sortRows(rows).map((r) => r.id)).toEqual([2, 1]);
  });

  it("does not mutate the input", () => {
    const rows = [row({ id: 1, score: 1 }), row({ id: 2, score: 9 })];
    sortRows(rows);
    expect(rows.map((r) => r.id)).toEqual([1, 2]);
  });
});

describe("filters", () => {
  it("filters by kind", () => {
    const rows = [row({ id: 1, kind: "new" }), row({ id: 2, kind: "update_suggestion" })];
    const out = filterRows(rows, { ...defaultFilters, kind: "update_suggestion" });
    expect(out.map((r) => r.id)).toEqual([2]);
  });

  it("filters by board", () => {
    const rows = [row({ id: 1, sourceBoard: "indeed" }), row({ id: 2, sourceBoard: "jobindex" })];
    const out = filterRows(rows, { ...defaultFilters, board: "jobindex" });
    expect(out.map((r) => r.id)).toEqual([2]);
  });

  it("filters by enrichment state", () => {
    const rows = [
      row({ id: 1, enrichmentState: "complete" }),
      row({ id: 2, enrichmentState: "failed" }),
    ];
    const out = filterRows(rows, { ...defaultFilters, enrichment: "failed" });
    expect(out.map((r) => r.id)).toEqual([2]);
  });

  it("filters by minimum score", () => {
    const rows = [row({ id: 1, score: 4 }), row({ id: 2, score: 9 })];
    const out = filterRows(rows, { ...defaultFilters, minScore: 7 });
    expect(out.map((r) => r.id)).toEqual([2]);
  });

  it("keeps invalid scores visible under a minimum-score filter", () => {
    // A row with no usable score is exactly the one a human needs to look at;
    // a numeric filter has no basis to hide it.
    const rows = [row({ id: 1, score: null, scoreState: "invalid" }), row({ id: 2, score: 2 })];
    const out = filterRows(rows, { ...defaultFilters, minScore: 7 });
    expect(out.map((r) => r.id)).toEqual([1]);
  });

  it("searches title, company, and board", () => {
    const rows = [
      row({ id: 1, title: "Rust Engineer", company: "Acme" }),
      row({ id: 2, title: "Barista", company: "Cafe" }),
    ];
    expect(filterRows(rows, { ...defaultFilters, query: "rust" }).map((r) => r.id)).toEqual([1]);
    expect(filterRows(rows, { ...defaultFilters, query: "cafe" }).map((r) => r.id)).toEqual([2]);
    expect(filterRows(rows, { ...defaultFilters, query: "  " }).map((r) => r.id)).toEqual([1, 2]);
  });

  it("combines filters and ordering in one pass", () => {
    const rows = [
      row({ id: 1, score: 9, sourceBoard: "jobindex" }),
      row({ id: 2, score: 8, sourceBoard: "indeed" }),
      row({ id: 3, score: 10, sourceBoard: "indeed" }),
    ];
    const out = visibleRows(rows, { ...defaultFilters, board: "indeed" });
    expect(out.map((r) => r.id)).toEqual([3, 2]);
  });

  it("offers only the boards actually present", () => {
    const rows = [
      row({ id: 1, sourceBoard: "indeed" }),
      row({ id: 2, sourceBoard: "jobindex" }),
      row({ id: 3, sourceBoard: "indeed" }),
      row({ id: 4, sourceBoard: null }),
    ];
    expect(boardOptions(rows)).toEqual(["indeed", "jobindex"]);
  });
});

describe("score chip", () => {
  it("renders an invalid score as a question mark, never a number", () => {
    expect(scoreLabel(row({ score: null, scoreState: "invalid" }))).toBe("?");
    expect(scoreLabel(row({ score: 3, scoreState: "invalid" }))).toBe("?");
  });

  it("renders a valid score as its number", () => {
    expect(scoreLabel(row({ score: 0 }))).toBe("0");
    expect(scoreLabel(row({ score: 10 }))).toBe("10");
  });
});

describe("badges", () => {
  it("flags incomplete enrichment", () => {
    expect(badgesFor(row({ enrichmentState: "partial" })).map((b) => b.kind)).toContain(
      "incompleteEnrichment",
    );
    expect(badgesFor(row({ enrichmentState: "skipped" })).map((b) => b.kind)).toContain(
      "incompleteEnrichment",
    );
  });

  it("distinguishes a failed enrichment from an incomplete one", () => {
    const kinds = badgesFor(row({ enrichmentState: "failed" })).map((b) => b.kind);
    expect(kinds).toContain("enrichmentFailed");
    expect(kinds).not.toContain("incompleteEnrichment");
  });

  it("says nothing when enrichment is complete", () => {
    expect(badgesFor(row({ enrichmentState: "complete" }))).toEqual([]);
  });

  it("flags suspicious content and near-duplicates", () => {
    const kinds = badgesFor(row({ suspicious: true, nearDuplicateOf: "fp-2" })).map((b) => b.kind);
    expect(kinds).toContain("suspicious");
    expect(kinds).toContain("nearDuplicate");
  });

  it("reports how many times a listing has been seen", () => {
    expect(badgesFor(row({ seenCount: 1 })).map((b) => b.kind)).not.toContain("seenAgain");
    const badge = badgesFor(row({ seenCount: 3 })).find((b) => b.kind === "seenAgain");
    expect(badge?.count).toBe(3);
  });

  it("marks an update suggestion", () => {
    expect(badgesFor(row({ kind: "update_suggestion" })).map((b) => b.kind)).toContain("update");
  });
});

describe("near-duplicates", () => {
  it("pairs them rather than merging them", () => {
    // Two listings with different strong keys are two jobs. Merging means
    // dismissing one silently buries the other (spec §5.1).
    const rows = [
      row({ id: 1, fingerprintId: "fp-kbh", nearDuplicateOf: "fp-aarhus" }),
      row({ id: 2, fingerprintId: "fp-aarhus" }),
    ];
    const pairs = pairNearDuplicates(rows);

    expect(visibleRows(rows, defaultFilters)).toHaveLength(2);
    expect(pairs.get(1)?.map((r) => r.fingerprintId)).toEqual(["fp-kbh", "fp-aarhus"]);
  });

  it("ignores a dangling counterpart", () => {
    const rows = [row({ id: 1, nearDuplicateOf: "fp-gone" })];
    expect(pairNearDuplicates(rows).size).toBe(0);
  });
});

describe("draft", () => {
  it("parses the prefill for the job form", () => {
    const draft = parseDraft(row({ draftJson: JSON.stringify({ company: "Acme", deadline: "x" }) }));
    expect(draft.company).toBe("Acme");
  });

  it("survives a malformed draft rather than blanking the inbox", () => {
    expect(parseDraft(row({ draftJson: "{not json" }))).toEqual({});
    expect(parseDraft(row({ draftJson: "[1,2]" }))).toEqual({});
  });

  it("returns listing text as text", () => {
    const html = "<script>alert(1)</script><b>bold</b>";
    const text = listingText(row({ draftJson: JSON.stringify({ raw_text: html }) }));
    // The value is returned verbatim for a text node; the point is that nothing
    // here produces markup, and the component renders it as a child, not as HTML.
    expect(text).toBe(html);
    expect(typeof text).toBe("string");
  });

  it("has no listing text when the draft carries none", () => {
    expect(listingText(row({ draftJson: "{}" }))).toBe("");
  });
});
