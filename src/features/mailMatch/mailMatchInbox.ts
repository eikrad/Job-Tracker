/**
 * Pure view logic for the Mail Match Inbox (spec §11.1).
 *
 * Kept separate from the component so filtering and badge rules are
 * asserted directly rather than through the DOM. Distinct from the Capture Inbox
 * (ADR 0003), which stays untouched.
 */

export type MailMatchRow = {
  id: number;
  fingerprintId: string;
  status: "pending" | "accepted" | "dismissed";
  jobId: number | null;
  /** `null` when `scoreState` is `invalid` — rendered as `?`, never as a number. */
  score: number | null;
  scoreReason: string | null;
  scoreState: "ok" | "invalid" | "skipped";
  suspicious: boolean;
  nearDuplicateOf: string | null;
  enrichmentState: "complete" | "partial" | "failed" | "skipped";
  enrichmentError: string | null;
  /**
   * The board never serves its listing page (Indeed), so the mail snippet is all
   * there is — expected, unlike a skipped or failed fetch.
   */
  snippetOnly: boolean;
  draftJson: string;
  sourceBoard: string | null;
  messageDate: string | null;
  listingUrl: string | null;
  title: string | null;
  company: string | null;
  seenCount: number;
  lastSeenAt: string;
  updatedAt: string;
};

export type MailMatchFilters = {
  minScore: number;
  board: string;
  enrichment: "all" | "complete" | "partial" | "failed" | "skipped";
  query: string;
};

export const defaultFilters: MailMatchFilters = {
  minScore: 0,
  board: "all",
  enrichment: "all",
  query: "",
};

function matchesQuery(row: MailMatchRow, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return [row.title, row.company, row.sourceBoard]
    .filter((v): v is string => Boolean(v))
    .some((v) => v.toLowerCase().includes(q));
}

/**
 * The rows the current filters let through, in the order they came.
 *
 * Ranking (score desc, invalid scores last, then most recently seen) is the backend's
 * job (`inbox.rs`), and every mutation here reloads from it, so nothing re-sorts.
 */
export function filterRows(rows: MailMatchRow[], filters: MailMatchFilters): MailMatchRow[] {
  return rows.filter((row) => {
    if (filters.board !== "all" && (row.sourceBoard ?? "") !== filters.board) return false;
    if (filters.enrichment !== "all" && row.enrichmentState !== filters.enrichment) return false;
    // An invalid score has no number, so a minimum-score filter cannot judge it.
    // Keep it visible: hiding it would bury exactly the rows that need a human.
    if (filters.minScore > 0 && row.score !== null && row.score < filters.minScore) return false;
    return matchesQuery(row, filters.query);
  });
}

/** Boards present in the data, for the filter dropdown. */
export function boardOptions(rows: MailMatchRow[]): string[] {
  const seen = new Set<string>();
  for (const row of rows) {
    if (row.sourceBoard) seen.add(row.sourceBoard);
  }
  return [...seen].sort();
}

export type BadgeKind =
  | "snippetOnly"
  | "incompleteEnrichment"
  | "enrichmentFailed"
  | "nearDuplicate"
  | "suspicious"
  | "seenAgain";

export type Badge = { kind: BadgeKind; count?: number };

/** Badges for one row, in the order they should read (spec §11.1). */
export function badgesFor(row: MailMatchRow): Badge[] {
  const badges: Badge[] = [];
  if (row.enrichmentState === "failed") badges.push({ kind: "enrichmentFailed" });
  else if (row.snippetOnly) badges.push({ kind: "snippetOnly" });
  else if (row.enrichmentState === "partial" || row.enrichmentState === "skipped") {
    badges.push({ kind: "incompleteEnrichment" });
  }
  if (row.nearDuplicateOf) badges.push({ kind: "nearDuplicate" });
  if (row.suspicious) badges.push({ kind: "suspicious" });
  if (row.seenCount > 1) badges.push({ kind: "seenAgain", count: row.seenCount });
  return badges;
}

/**
 * What the score chip shows. `?` for an invalid score — a number the user might
 * trust is exactly what an unparseable model answer must not become.
 */
export function scoreLabel(row: MailMatchRow): string {
  if (row.scoreState === "invalid" || row.score === null) return "?";
  return String(row.score);
}

/**
 * Near-duplicates are shown as a pair, never merged (spec §5.1): two listings with
 * different strong keys really are two jobs, and merging them means dismissing one
 * buries the other.
 */
export function pairNearDuplicates(rows: MailMatchRow[]): Map<number, MailMatchRow[]> {
  const byFingerprint = new Map<string, MailMatchRow>();
  for (const row of rows) byFingerprint.set(row.fingerprintId, row);

  const pairs = new Map<number, MailMatchRow[]>();
  for (const row of rows) {
    if (!row.nearDuplicateOf) continue;
    const counterpart = byFingerprint.get(row.nearDuplicateOf);
    if (counterpart) pairs.set(row.id, [row, counterpart]);
  }
  return pairs;
}

/** Parsed draft, used to prefill the job form. Never rendered as HTML. */
export function parseDraft(row: MailMatchRow): Record<string, unknown> {
  try {
    const parsed: unknown = JSON.parse(row.draftJson);
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    // A malformed draft must not blank the inbox; the row still shows its score.
  }
  return {};
}

/** The extracted listing text, as text. */
export function listingText(row: MailMatchRow): string {
  const draft = parseDraft(row);
  const raw = draft.raw_text;
  return typeof raw === "string" ? raw : "";
}
