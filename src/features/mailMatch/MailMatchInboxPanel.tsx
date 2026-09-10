/**
 * Mail Match Inbox (spec §11.1) — its own surface, separate from the Capture Inbox
 * (ADR 0003).
 *
 * Two rules run through the whole component:
 *
 * - **Accept never writes silently.** It hands a prefilled draft to the job form and
 *   the user submits it. An update goes through a diff first.
 * - **Listing text is text.** Bodies and pages here came out of email. Nothing in this
 *   file uses `dangerouslySetInnerHTML`, and `MailMatchInboxPanel.test.tsx` asserts it.
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import { en } from "../../i18n/en";
import type { NewJob } from "../../lib/types";
import {
  mailMatchAcceptUpdate,
  mailMatchDismiss,
  mailMatchList,
  mailMatchListDismissed,
  mailMatchPreviewUpdate,
  mailMatchRestore,
  mailScanListRuns,
  type DismissedRow,
  type RunRow,
  type UpdatePreview,
} from "./mailMatchApi";
import {
  badgesFor,
  boardOptions,
  defaultFilters,
  listingText,
  pairNearDuplicates,
  parseDraft,
  scoreLabel,
  visibleRows,
  type Badge,
  type MailMatchFilters,
  type MailMatchRow,
} from "./mailMatchInbox";
import { runViewFromRow, type RunView } from "./runSummary";
import { RunSummaryCard } from "./RunSummaryCard";

const t = en.mailMatch;

type Tab = "pending" | "dismissed" | "history";

export type MailMatchInboxPanelProps = {
  /** Opens the job form prefilled — the only way a match becomes a Job. */
  onAcceptDraft: (inboxId: number, draft: Partial<NewJob>) => void;
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
  previewUpdate: typeof mailMatchPreviewUpdate;
  acceptUpdate: typeof mailMatchAcceptUpdate;
};

const realApi: MailMatchApi = {
  list: mailMatchList,
  listDismissed: mailMatchListDismissed,
  listRuns: mailScanListRuns,
  dismiss: mailMatchDismiss,
  restore: mailMatchRestore,
  previewUpdate: mailMatchPreviewUpdate,
  acceptUpdate: mailMatchAcceptUpdate,
};

const badgeLabels: Record<Badge["kind"], string> = {
  update: t.badgeUpdate,
  incompleteEnrichment: t.badgeIncompleteEnrichment,
  enrichmentFailed: t.badgeEnrichmentFailed,
  nearDuplicate: t.badgeNearDuplicate,
  suspicious: t.badgeSuspicious,
  seenAgain: "",
};

function BadgeList({ badges }: { badges: Badge[] }) {
  if (badges.length === 0) return null;
  return (
    <ul className="mail-match__badges">
      {badges.map((badge) => (
        <li
          key={badge.kind}
          className={`mail-match__badge mail-match__badge--${badge.kind}`}
          title={badge.kind === "suspicious" ? t.badgeSuspiciousTitle : undefined}
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

/** The recomputed diff, including the "job changed" state from C2. */
function UpdateDiff({
  preview,
  onApply,
  onCancel,
}: {
  preview: UpdatePreview;
  onApply: () => void;
  onCancel: () => void;
}) {
  const applicable = preview.fields.filter((f) => f.applicable);
  return (
    <section className="mail-match__diff" aria-label={t.diffTitle}>
      <h4>{t.diffTitle}</h4>
      {preview.jobChangedSinceScan ? (
        <p className="mail-match__diff-warning" role="status">
          {t.diffJobChanged}
        </p>
      ) : null}
      {preview.fields.length === 0 || applicable.length === 0 ? (
        <p className="mail-match__empty">{t.diffNothingToDo}</p>
      ) : null}
      <ul className="mail-match__diff-list">
        {preview.fields.map((field) => (
          <li
            key={field.field}
            className={field.applicable ? "is-applicable" : "is-skipped"}
          >
            <span className="mail-match__diff-field">{t.diffFieldLabel(field.field)}</span>
            <span className="mail-match__diff-value">{field.suggested}</span>
            <span className="mail-match__diff-note">
              {field.applicable
                ? t.diffWillWrite
                : `${t.diffSkipped} — ${t.diffCurrent(field.current ?? "")}`}
            </span>
          </li>
        ))}
      </ul>
      <div className="mail-match__diff-actions">
        <button type="button" onClick={onApply} disabled={applicable.length === 0}>
          {t.diffApply(applicable.length)}
        </button>
        <button type="button" onClick={onCancel}>
          {t.diffCancel}
        </button>
      </div>
    </section>
  );
}

export function MailMatchInboxPanel({
  onAcceptDraft,
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
  const [preview, setPreview] = useState<UpdatePreview | null>(null);
  const [dismissing, setDismissing] = useState<number | null>(null);
  const [dismissReason, setDismissReason] = useState("");
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

  const shown = useMemo(() => visibleRows(rows, filters), [rows, filters]);
  const pairs = useMemo(() => pairNearDuplicates(rows), [rows]);
  const boards = useMemo(() => boardOptions(rows), [rows]);
  const filtersActive = useMemo(
    () => JSON.stringify(filters) !== JSON.stringify(defaultFilters),
    [filters],
  );

  function acceptRow(row: MailMatchRow) {
    // Never a silent write: hand the draft to the form and let the user submit.
    onAcceptDraft(row.id, parseDraft(row) as Partial<NewJob>);
  }

  async function reviewUpdate(row: MailMatchRow) {
    try {
      setPreview(await client.previewUpdate(row.id));
    } catch (e) {
      setError(String(e));
    }
  }

  async function applyUpdate() {
    if (!preview) return;
    try {
      await client.acceptUpdate(preview.inboxId);
      setPreview(null);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function confirmDismiss(row: MailMatchRow) {
    try {
      await client.dismiss(row.id, dismissReason.trim() || undefined);
      setDismissing(null);
      setDismissReason("");
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
              {t.filterKind}
              <select
                value={filters.kind}
                onChange={(e) =>
                  setFilters({ ...filters, kind: e.target.value as MailMatchFilters["kind"] })
                }
              >
                <option value="all">{t.filterKindAll}</option>
                <option value="new">{t.filterKindNew}</option>
                <option value="update_suggestion">{t.filterKindUpdate}</option>
              </select>
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
                      {row.kind === "update_suggestion" ? (
                        <button type="button" onClick={() => void reviewUpdate(row)}>
                          {t.acceptUpdate}
                        </button>
                      ) : (
                        <button type="button" onClick={() => acceptRow(row)} title={t.acceptHint}>
                          {t.accept}
                        </button>
                      )}
                      <button type="button" onClick={() => setDismissing(row.id)}>
                        {t.dismiss}
                      </button>
                    </div>

                    {dismissing === row.id ? (
                      <div className="mail-match__dismiss">
                        <label>
                          {t.dismissReasonLabel}
                          <input
                            type="text"
                            value={dismissReason}
                            placeholder={t.dismissReasonPlaceholder}
                            onChange={(e) => setDismissReason(e.target.value)}
                          />
                        </label>
                        <button type="button" onClick={() => void confirmDismiss(row)}>
                          {t.dismissConfirm}
                        </button>
                        <button type="button" onClick={() => setDismissing(null)}>
                          {t.dismissCancel}
                        </button>
                      </div>
                    ) : null}
                  </li>
                );
              })}
            </ul>
          )}

          {preview ? (
            <UpdateDiff
              preview={preview}
              onApply={() => void applyUpdate()}
              onCancel={() => setPreview(null)}
            />
          ) : null}
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
