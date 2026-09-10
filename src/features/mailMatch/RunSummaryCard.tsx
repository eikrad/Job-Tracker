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
          <progress value={percent ?? undefined} max={100} />
          <span>{t.scanProgressListings(stats.listingsCommitted)}</span>
          <span>{t.scanProgressMessages(stats.messagesSeen)}</span>
          {onCancel ? (
            <button type="button" onClick={onCancel}>
              {t.scanCancel}
            </button>
          ) : null}
        </div>
      ) : null}

      <ul className="run-summary__counters">
        <li>{t.summaryNew(stats.inboxNew)}</li>
        <li>{t.summaryUpdates(stats.updates)}</li>
        <li>{t.summaryUnderCutoff(stats.underCutoff)}</li>
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
        {stats.enrichmentFailures > 0 ? (
          <li>{t.summaryEnrichmentFailures(stats.enrichmentFailures)}</li>
        ) : null}
        <li>{t.summaryCalls(stats.llmCalls)}</li>
      </ul>

      {stats.budgetExhausted ? (
        <p className="run-summary__notice" role="status">
          {t.summaryBudgetExhausted}
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
