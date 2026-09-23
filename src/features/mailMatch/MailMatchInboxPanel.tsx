/**
 * Mail Match Inbox (spec §11.1) — its own surface, separate from the Capture Inbox
 * (ADR 0003).
 *
 * Two rules run through the whole component:
 *
 * - **Accept is one click, and undoable.** A match becomes an Interesting Job
 *   straight from its draft; the notice that follows offers Open and Undo.
 * - **Listing text is text.** Bodies and pages here came out of email. Nothing in this
 *   file uses `dangerouslySetInnerHTML`, and `MailMatchInboxPanel.test.tsx` asserts it.
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import { en } from "../../i18n/en";
import {
  mailMatchAcceptNew,
  mailMatchDismiss,
  mailMatchList,
  mailMatchListDismissed,
  mailMatchRestore,
  mailMatchUndoAccept,
  mailScanListRuns,
  type DismissedRow,
  type RunRow,
} from "./mailMatchApi";
import {
  badgesFor,
  boardOptions,
  defaultFilters,
  filterRows,
  listingText,
  pairNearDuplicates,
  scoreLabel,
  type Badge,
  type MailMatchFilters,
  type MailMatchRow,
} from "./mailMatchInbox";
import { runViewFromRow, type RunView } from "./runSummary";
import { RunSummaryCard } from "./RunSummaryCard";

const t = en.mailMatch;

type Tab = "pending" | "dismissed" | "history";

export type MailMatchInboxPanelProps = {
  /** Called after an accept or undo, so the job board can reload. */
  onJobsChanged?: () => void;
  /** Opens a Job's detail page — the notice's Open button. */
  onOpenJob?: (jobId: number) => void;
  /** Bump after a scan finishes so pending/history reload. */
  reloadToken?: number;
  /** When the page already shows the title (e.g. next to Scan control). */
  hidePageHeading?: boolean;
  /** Injected in tests; defaults to the real commands. */
  api?: Partial<MailMatchApi>;
};

type MailMatchApi = {
  list: typeof mailMatchList;
  listDismissed: typeof mailMatchListDismissed;
  listRuns: typeof mailScanListRuns;
  dismiss: typeof mailMatchDismiss;
  restore: typeof mailMatchRestore;
  acceptNew: typeof mailMatchAcceptNew;
  undoAccept: typeof mailMatchUndoAccept;
};

/** What the last accept did, kept outside the row: the row leaves the list on accept. */
type AcceptNotice =
  | { kind: "accepted"; inboxId: number; jobId: number; title: string }
  | { kind: "undone" };

const realApi: MailMatchApi = {
  list: mailMatchList,
  listDismissed: mailMatchListDismissed,
  listRuns: mailScanListRuns,
  dismiss: mailMatchDismiss,
  restore: mailMatchRestore,
  acceptNew: mailMatchAcceptNew,
  undoAccept: mailMatchUndoAccept,
};

const badgeLabels: Record<Badge["kind"], string> = {
  snippetOnly: t.badgeSnippetOnly,
  incompleteEnrichment: t.badgeIncompleteEnrichment,
  enrichmentFailed: t.badgeEnrichmentFailed,
  nearDuplicate: t.badgeNearDuplicate,
  suspicious: t.badgeSuspicious,
  seenAgain: "",
};

const badgeTitles: Partial<Record<Badge["kind"], string>> = {
  snippetOnly: t.badgeSnippetOnlyTitle,
  suspicious: t.badgeSuspiciousTitle,
};

function BadgeList({ badges }: { badges: Badge[] }) {
  if (badges.length === 0) return null;
  return (
    <ul className="mail-match__badges">
      {badges.map((badge) => (
        <li
          key={badge.kind}
          className={`mail-match__badge mail-match__badge--${badge.kind}`}
          title={badgeTitles[badge.kind]}
        >
          {badge.kind === "seenAgain" ? t.seenTimes(badge.count ?? 2) : badgeLabels[badge.kind]}
        </li>
      ))}
    </ul>
  );
}

function ScoreChip({ row }: { row: MailMatchRow }) {
  const label = scoreLabel(row);
  const invalid = label === "?";
  return (
    <span
      className={`mail-match__score${invalid ? " mail-match__score--invalid" : ""}`}
      aria-label={t.scoreAria(label)}
      title={invalid ? t.scoreInvalidTitle : (row.scoreReason ?? undefined)}
    >
      {label}
    </span>
  );
}

export function MailMatchInboxPanel({
  onJobsChanged,
  onOpenJob,
  reloadToken = 0,
  hidePageHeading = false,
  api,
}: MailMatchInboxPanelProps) {
  const client = useMemo<MailMatchApi>(() => ({ ...realApi, ...api }), [api]);

  const [tab, setTab] = useState<Tab>("pending");
  const [rows, setRows] = useState<MailMatchRow[]>([]);
  const [dismissed, setDismissed] = useState<DismissedRow[]>([]);
  const [runs, setRuns] = useState<RunRow[]>([]);
  const [filters, setFilters] = useState<MailMatchFilters>(defaultFilters);
  const [expanded, setExpanded] = useState<number | null>(null);
  const [notice, setNotice] = useState<AcceptNotice | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(
    () =>
      Promise.all([client.list("pending"), client.listDismissed(), client.listRuns(20)])
        .then(([pending, gone, history]) => {
          setRows(pending);
          setDismissed(gone);
          setRuns(history);
          setError(null);
        })
        .catch((e: unknown) => setError(String(e))),
    [client],
  );

  useEffect(() => {
    void refresh();
  }, [refresh, reloadToken]);

  const shown = useMemo(() => filterRows(rows, filters), [rows, filters]);
  const pairs = useMemo(() => pairNearDuplicates(rows), [rows]);
  const boards = useMemo(() => boardOptions(rows), [rows]);
  const filtersActive = useMemo(
    () => JSON.stringify(filters) !== JSON.stringify(defaultFilters),
    [filters],
  );

  async function acceptRow(row: MailMatchRow) {
    try {
      const outcome = await client.acceptNew(row.id);
      // `created: false` means a double click lost the race: the first one already
      // showed its notice.
      if (outcome.created) {
        setNotice({
          kind: "accepted",
          inboxId: row.id,
          jobId: outcome.jobId,
          title: row.title ?? en.common.untitled,
        });
      }
      onJobsChanged?.();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function undoAccept(inboxId: number) {
    try {
      await client.undoAccept(inboxId);
      setNotice({ kind: "undone" });
      onJobsChanged?.();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function dismissRow(row: MailMatchRow) {
    // One click: the Dismissed tab is the undo, so there is nothing to confirm.
    try {
      await client.dismiss(row.id);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function restoreOne(fingerprintId: string) {
    try {
      await client.restore(fingerprintId);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="mail-match" aria-label={t.title}>
      {hidePageHeading ? null : (
        <header className="mail-match__header">
          <h2>{t.title}</h2>
          <p className="mail-match__subtitle">{t.subtitle}</p>
        </header>
      )}

      {error ? (
        <p className="mail-match__error" role="alert">
          {error}
        </p>
      ) : null}

      {notice ? (
        <div className="mail-match__notice" role="status">
          {notice.kind === "accepted" ? (
            <>
              <span>{t.acceptedNotice(notice.title)}</span>
              {onOpenJob ? (
                <button type="button" onClick={() => onOpenJob(notice.jobId)}>
                  {t.acceptedOpen}
                </button>
              ) : null}
              <button type="button" onClick={() => void undoAccept(notice.inboxId)}>
                {t.acceptedUndo}
              </button>
            </>
          ) : (
            <span>{t.undoneNotice}</span>
          )}
        </div>
      ) : null}

      <div className="mail-match__tabs" role="tablist">
        {(
          [
            ["pending", t.tabPending, rows.length],
            ["dismissed", t.tabDismissed, dismissed.length],
            ["history", t.tabHistory, runs.length],
          ] as const
        ).map(([id, label, count]) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            className={tab === id ? "is-active" : undefined}
            onClick={() => setTab(id)}
          >
            {label} <span className="mail-match__tab-count">{t.tabCount(count)}</span>
          </button>
        ))}
      </div>

      {tab === "pending" ? (
        <>
          <div className="mail-match__filters">
            <label>
              {t.searchLabel}
              <input
                type="search"
                value={filters.query}
                placeholder={t.searchPlaceholder}
                onChange={(e) => setFilters({ ...filters, query: e.target.value })}
              />
            </label>
            <label>
              {t.filterBoard}
              <select
                value={filters.board}
                onChange={(e) => setFilters({ ...filters, board: e.target.value })}
              >
                <option value="all">{t.filterBoardAll}</option>
                {boards.map((board) => (
                  <option key={board} value={board}>
                    {board}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t.filterEnrichment}
              <select
                value={filters.enrichment}
                onChange={(e) =>
                  setFilters({
                    ...filters,
                    enrichment: e.target.value as MailMatchFilters["enrichment"],
                  })
                }
              >
                <option value="all">{t.filterEnrichmentAll}</option>
                <option value="complete">{t.filterEnrichmentComplete}</option>
                <option value="partial">{t.filterEnrichmentPartial}</option>
                <option value="failed">{t.filterEnrichmentFailed}</option>
                <option value="skipped">{t.filterEnrichmentSkipped}</option>
              </select>
            </label>
            <label>
              {t.filterMinScore}
              <select
                value={String(filters.minScore)}
                onChange={(e) => setFilters({ ...filters, minScore: Number(e.target.value) })}
              >
                <option value="0">{t.filterMinScoreAny}</option>
                {[5, 6, 7, 8, 9, 10].map((n) => (
                  <option key={n} value={String(n)}>
                    {n}+
                  </option>
                ))}
              </select>
            </label>
            {filtersActive ? (
              <button type="button" onClick={() => setFilters(defaultFilters)}>
                {t.clearFilters}
              </button>
            ) : null}
          </div>

          {shown.length === 0 ? (
            <p className="mail-match__empty">
              {filtersActive || rows.length > 0 ? t.emptyPendingFiltered : t.emptyPending}
            </p>
          ) : (
            <ul className="mail-match__list">
              {shown.map((row) => {
                const pair = pairs.get(row.id);
                const isOpen = expanded === row.id;
                return (
                  <li key={row.id} className="mail-match__row">
                    <div className="mail-match__row-main">
                      <ScoreChip row={row} />
                      <button
                        type="button"
                        className="mail-match__row-title"
                        aria-expanded={isOpen}
                        onClick={() => setExpanded(isOpen ? null : row.id)}
                      >
                        <strong>{row.title ?? en.common.untitled}</strong>
                        <span>{row.company ?? en.common.dash}</span>
                        <span className="mail-match__board">
                          {row.sourceBoard ?? t.noBoard}
                        </span>
                      </button>
                      <BadgeList badges={badgesFor(row)} />
                    </div>

                    {pair ? <p className="mail-match__pair-hint">{t.nearDuplicateHint}</p> : null}

                    {isOpen ? (
                      <div className="mail-match__detail">
                        <section>
                          <h4>{t.detailListingText}</h4>
                          <p className="mail-match__hint">{t.detailListingTextHint}</p>
                          {/* Rendered as a text child — never as HTML. */}
                          <pre className="mail-match__listing-text">{listingText(row)}</pre>
                        </section>
                        {row.scoreReason ? (
                          <p className="mail-match__reason">{row.scoreReason}</p>
                        ) : null}
                        {row.enrichmentError ? (
                          <p className="mail-match__reason">
                            {t.enrichmentReason(row.enrichmentError)}
                          </p>
                        ) : null}
                        {row.listingUrl ? (
                          <a href={row.listingUrl} target="_blank" rel="noreferrer noopener">
                            {t.openListing}
                          </a>
                        ) : null}
                      </div>
                    ) : null}

                    <div className="mail-match__actions">
                      <button
                        type="button"
                        onClick={() => void acceptRow(row)}
                        title={t.acceptHint}
                      >
                        {t.accept}
                      </button>
                      <button type="button" onClick={() => void dismissRow(row)}>
                        {t.dismiss}
                      </button>
                    </div>

                  </li>
                );
              })}
            </ul>
          )}

        </>
      ) : null}

      {tab === "dismissed" ? (
        dismissed.length === 0 ? (
          <p className="mail-match__empty">{t.emptyDismissed}</p>
        ) : (
          <ul className="mail-match__list">
            {dismissed.map((item) => (
              <li key={item.fingerprintId} className="mail-match__row">
                <div className="mail-match__row-main">
                  <span>
                    <strong>{item.title ?? en.common.untitled}</strong>{" "}
                    <span>{item.company ?? en.common.dash}</span>
                  </span>
                  <span className="mail-match__hint">
                    {item.reason ?? t.dismissedNoReason} · {t.restoredAt(item.dismissedAt)}
                  </span>
                </div>
                <div className="mail-match__actions">
                  <button type="button" onClick={() => void restoreOne(item.fingerprintId)}>
                    {t.restore}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )
      ) : null}

      {tab === "history" ? (
        runs.length === 0 ? (
          <p className="mail-match__empty">{t.emptyHistory}</p>
        ) : (
          <ul className="mail-match__list">
            {runs.map((run) => {
              const view: RunView = runViewFromRow(run);
              return (
                <li key={run.runId} className="mail-match__row">
                  {/* Same component as the live run — one renderer, two sources. */}
                  <RunSummaryCard
                    view={view}
                    onShowSuppressed={() => setTab("dismissed")}
                  />
                </li>
              );
            })}
          </ul>
        )
      ) : null}
    </section>
  );
}
