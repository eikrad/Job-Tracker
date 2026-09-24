/**
 * One renderer for a scan run, live or finished (spec §8.4, §11.2).
 *
 * History passes a `RunView` built from `stats_json`; the scan control passes one built
 * from `mail-scan://progress` events. Both go through this component, so a finished run
 * cannot drift into showing something a live one never did.
 */

import { en } from "../../i18n/en";
import { isTerminal, progressPercent, type RunView } from "./runSummary";
import { errorMessage } from "./runErrors";

function phaseLabel(stats: RunView["stats"]): string {
  if (stats.messagesSeen === 0 && stats.listingsCommitted === 0) return t.scanPhaseOpening;
  if (stats.llmCalls > 0) return t.scanPhaseScoring;
  return t.scanPhaseReading;
}

const t = en.mailMatch;

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
              {t.scanProgressMessages(stats.messagesSeen)}
            </span>
            {onCancel ? (
              <button type="button" onClick={onCancel}>
                {t.scanCancel}
              </button>
            ) : null}
          </div>
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
