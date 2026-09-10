/**
 * Pure view logic for the Mail Match Inbox (spec §11.1).
 *
 * Kept separate from the component so ordering, filtering, and badge rules are
 * asserted directly rather than through the DOM. Distinct from the Capture Inbox
 * (ADR 0003), which stays untouched.
 */

export type MailMatchRow = {
  id: number;
  fingerprintId: string;
  kind: "new" | "update_suggestion";
  status: "pending" | "accepted" | "dismissed" | "superseded";
  jobId: number | null;
  /** `null` when `scoreState` is `invalid` — rendered as `?`, never as a number. */
  score: number | null;
  scoreReason: string | null;
  scoreState: "ok" | "invalid" | "skipped";
  suspicious: boolean;
  nearDuplicateOf: string | null;
  enrichmentState: "complete" | "partial" | "failed" | "skipped";
  enrichmentError: string | null;
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
  kind: "all" | "new" | "update_suggestion";
  minScore: number;
  board: string;
  enrichment: "all" | "complete" | "partial" | "failed" | "skipped";
  query: string;
};

export const defaultFilters: MailMatchFilters = {
  kind: "all",
  minScore: 0,
  board: "all",
  enrichment: "all",
  query: "",
};

/**
 * Score desc, then most recently seen (spec §11.1).
 *
 * An `invalid` score has no number to rank by, so it sorts below every real score
 * rather than above them — the backend orders the same way, and this keeps a
 * client-side re-sort from contradicting it.
 */
export function sortRows(rows: MailMatchRow[]): MailMatchRow[] {
  return [...rows].sort((a, b) => {
    const sa = a.score ?? -1;
    const sb = b.score ?? -1;
    if (sa !== sb) return sb - sa;
    if (a.lastSeenAt !== b.lastSeenAt) return a.lastSeenAt < b.lastSeenAt ? 1 : -1;
    return b.id - a.id;
  });
}

function matchesQuery(row: MailMatchRow, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return [row.title, row.company, row.sourceBoard]
    .filter((v): v is string => Boolean(v))
    .some((v) => v.toLowerCase().includes(q));
}

export function filterRows(rows: MailMatchRow[], filters: MailMatchFilters): MailMatchRow[] {
  return rows.filter((row) => {
    if (filters.kind !== "all" && row.kind !== filters.kind) return false;
    if (filters.board !== "all" && (row.sourceBoard ?? "") !== filters.board) return false;
    if (filters.enrichment !== "all" && row.enrichmentState !== filters.enrichment) return false;
    // An invalid score has no number, so a minimum-score filter cannot judge it.
    // Keep it visible: hiding it would bury exactly the rows that need a human.
    if (filters.minScore > 0 && row.score !== null && row.score < filters.minScore) return false;
    return matchesQuery(row, filters.query);
  });
}

export function visibleRows(rows: MailMatchRow[], filters: MailMatchFilters): MailMatchRow[] {
  return sortRows(filterRows(rows, filters));
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
  | "update"
  | "incompleteEnrichment"
  | "enrichmentFailed"
  | "nearDuplicate"
  | "suspicious"
  | "seenAgain";

export type Badge = { kind: BadgeKind; count?: number };

/** Badges for one row, in the order they should read (spec §11.1). */
export function badgesFor(row: MailMatchRow): Badge[] {
  const badges: Badge[] = [];
  if (row.kind === "update_suggestion") badges.push({ kind: "update" });
  if (row.enrichmentState === "failed") badges.push({ kind: "enrichmentFailed" });
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
