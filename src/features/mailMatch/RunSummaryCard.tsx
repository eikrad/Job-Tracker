/**
 * One renderer for a scan run, live or finished (spec §8.4, §11.2).
 *
 * History passes a `RunView` built from `stats_json`; the scan control passes one built
 * from `mail-scan://progress` events. Both go through this component, so a finished run
 * cannot drift into showing something a live one never did.
 */

import { useEffect, useState } from "react";
import { en } from "../../i18n/en";
import { idleSeconds, isTerminal, progressPercent, type RunView } from "./runSummary";
import { errorMessage } from "./runErrors";

const t = en.mailMatch;

/** A live run whose counters have not moved for this long gets a "still running" note. */
const QUIET_AFTER_SECONDS = 20;

function phaseLabel(stats: RunView["stats"]): string {
  if (stats.messagesParsed === 0 && stats.listingsCommitted === 0) return t.scanPhaseOpening;
  if (stats.llmCalls > 0) return t.scanPhaseScoring;
  return t.scanPhaseReading;
}

/** The current time, re-read every few seconds while `live`, so idle time counts up. */
function useClock(live: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!live) return;
    const id = window.setInterval(() => setNow(Date.now()), 5_000);
    return () => window.clearInterval(id);
  }, [live]);
  return now;
}

const statusLabels: Record<RunView["status"], string> = {
  running: t.statusRunning,
  completed: t.statusCompleted,
  cancelled: t.statusCancelled,
  failed: t.statusFailed,
};

export type RunSummaryCardProps = {
  view: RunView;
  onShowSuppressed?: () => void;
  onContinue?: () => void;
  onCancel?: () => void;
};

export function RunSummaryCard({
  view,
  onShowSuppressed,
  onContinue,
  onCancel,
}: RunSummaryCardProps) {
  const { stats } = view;
  const percent = progressPercent(view);
  const terminal = isTerminal(view);
  const failure = errorMessage(view.errorCode);
  const now = useClock(!terminal);
  const idle = idleSeconds(view, now);

  return (
    <article className={`run-summary run-summary--${view.status}`}>
      <header className="run-summary__header">
        <span className={`run-summary__status run-summary__status--${view.status}`}>
          {statusLabels[view.status]}
        </span>
        {view.startedAt ? (
          <time className="run-summary__time">{view.startedAt}</time>
        ) : null}
        {view.modelId ? <span className="run-summary__model">{view.modelId}</span> : null}
      </header>

      {!terminal ? (
        <div className="run-summary__progress">
          <p className="run-summary__phase" role="status">
            <span className="run-summary__pulse" aria-hidden="true" />
            {phaseLabel(stats)}
          </p>
          <progress value={percent ?? undefined} max={100} />
          <div className="run-summary__progress-row">
            <span>
              {t.scanProgressListings(stats.listingsCommitted)} ·{" "}
              {t.scanProgressMessages(stats.messagesParsed)}
            </span>
            {onCancel ? (
              <button type="button" onClick={onCancel}>
                {t.scanCancel}
              </button>
            ) : null}
          </div>
          {idle !== null && idle >= QUIET_AFTER_SECONDS ? (
            <p className="run-summary__quiet">{t.scanStillWorking(idle)}</p>
          ) : null}
        </div>
      ) : null}

      <ul className="run-summary__counters">
        <li>{t.summaryNew(stats.inboxNew)}</li>
        <li>{t.summaryUnderCutoff(stats.underCutoff)}</li>
        {stats.alreadyTracked > 0 ? <li>{t.summaryAlreadyTracked(stats.alreadyTracked)}</li> : null}
        <li>
          {t.summarySuppressed(stats.suppressedByDismissal)}
          {stats.suppressedByDismissal > 0 && onShowSuppressed ? (
            <>
              {" "}
              <button type="button" className="run-summary__link" onClick={onShowSuppressed}>
                {t.summarySuppressedLink}
              </button>
            </>
          ) : null}
        </li>
        {stats.skippedByTitle > 0 ? <li>{t.summarySkippedByTitle(stats.skippedByTitle)}</li> : null}
        {stats.closed > 0 ? <li>{t.summaryClosed(stats.closed)}</li> : null}
        {stats.enrichmentFailures > 0 ? (
          <li>{t.summaryEnrichmentFailures(stats.enrichmentFailures)}</li>
        ) : null}
        <li>{t.summaryCalls(stats.llmCalls)}</li>
      </ul>

      {stats.budgetExhausted || stats.listingLimitReached ? (
        <p className="run-summary__notice" role="status">
          {stats.budgetExhausted ? t.summaryBudgetExhausted : t.summaryListingLimitReached}
          {onContinue ? (
            <>
              {" "}
              <button type="button" onClick={onContinue}>
                {t.summaryContinue}
              </button>
            </>
          ) : null}
        </p>
      ) : null}

      {failure ? (
        <p className="run-summary__error" role="alert">
          {failure}
        </p>
      ) : null}
    </article>
  );
}
