/**
 * Scan control (spec §11.2): pre-run sheet, live progress, cancel, summary.
 *
 * The pre-run sheet exists because a scan spends money. It shows the **resolved**
 * folder paths (§6.4), the model, the cutoff, and the call estimate before the user
 * commits — never after.
 */

import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { en } from "../../i18n/en";
import {
  mailScanCancel,
  mailScanEstimate,
  mailScanStart,
  type MailSource,
  type ScanEstimate,
} from "./mailMatchApi";
import { applyProgress, isTerminal, type ProgressEvent, type RunView } from "./runSummary";
import { RunSummaryCard } from "./RunSummaryCard";

const t = en.mailMatch;

export type ScanControlProps = {
  sources: MailSource[];
  provider?: string;
  cutoff: number;
  sinceDays: number;
  maxCalls: number;
  /** Refresh the inbox when a run reaches a terminal state. */
  onRunFinished?: () => void;
};

export function ScanControl({
  sources,
  provider,
  cutoff,
  sinceDays,
  maxCalls,
  onRunFinished,
}: ScanControlProps) {
  const [sheetOpen, setSheetOpen] = useState(false);
  const [estimate, setEstimate] = useState<ScanEstimate | null>(null);
  const [view, setView] = useState<RunView | null>(null);
  const [error, setError] = useState<string | null>(null);

  const running = view !== null && !isTerminal(view);

  useEffect(() => {
    const unlisten = listen<ProgressEvent>("mail-scan://progress", (event) => {
      setView((current) => applyProgress(current, event.payload));
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, []);

  useEffect(() => {
    if (view && isTerminal(view)) onRunFinished?.();
    // Only fire on a status transition, not on every counter tick.
  }, [view?.status, view?.runId]); // eslint-disable-line react-hooks/exhaustive-deps

  const openSheet = useCallback(async () => {
    setError(null);
    setSheetOpen(true);
    try {
      setEstimate(
        await mailScanEstimate({
          provider,
          // Coarse prior for a first look; the run's own budget is the real bound.
          expectedListings: 120,
          maxCalls,
        }),
      );
    } catch (e) {
      setError(String(e));
    }
  }, [provider, maxCalls]);

  const profilesReady =
    estimate?.profileShort.configured === true && estimate?.profileFull.configured === true;
  const canStart = sources.length > 0 && profilesReady;

  async function start() {
    try {
      setSheetOpen(false);
      setView(null);
      await mailScanStart({ sources, provider, cutoff, sinceDays, maxCalls });
    } catch (e) {
      setError(String(e));
    }
  }

  async function cancel() {
    try {
      await mailScanCancel();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="scan-control">
      <button
        type="button"
        className="btn btnPrimary"
        onClick={() => void openSheet()}
        disabled={running}
      >
        {running ? t.scanRunning : t.scanButton}
      </button>

      {error ? (
        <p className="scan-control__error" role="alert">
          {error}
        </p>
      ) : null}

      {sheetOpen ? (
        <div className="scan-control__sheet" role="dialog" aria-label={t.scanPreflightTitle}>
          <h3>{t.scanPreflightTitle}</h3>

          <h4>{t.scanPreflightFolders}</h4>
          {sources.length === 0 ? (
            <p className="scan-control__warning">{t.scanPreflightNoFolders}</p>
          ) : (
            <ul>
              {sources.map((source) => (
                // The resolved path, so a symlinked folder is visible before it is read.
                <li key={source.id}>
                  <code>{source.path}</code> <span>({source.kind})</span>
                </li>
              ))}
            </ul>
          )}

          {estimate ? (
            <dl className="scan-control__facts">
              <dt>{t.scanPreflightModel}</dt>
              <dd>{estimate.modelId}</dd>
              <dt>{t.scanPreflightCutoff}</dt>
              <dd>{cutoff}</dd>
            </dl>
          ) : null}

          {estimate ? (
            <p className="scan-control__estimate">
              {t.scanPreflightEstimate(estimate.estimate)}
            </p>
          ) : null}
          {estimate?.estimate.overBudget ? (
            <p className="scan-control__warning">{t.scanPreflightOverBudget}</p>
          ) : null}
          {estimate && !profilesReady ? (
            <p className="scan-control__warning">{t.scanPreflightProfilesMissing}</p>
          ) : null}

          <div className="scan-control__sheet-actions">
            <button
              type="button"
              className="btn btnPrimary"
              onClick={() => void start()}
              disabled={!canStart}
            >
              {t.scanPreflightStart}
            </button>
            <button type="button" className="btn btnGhost" onClick={() => setSheetOpen(false)}>
              {t.scanPreflightCancel}
            </button>
          </div>
        </div>
      ) : null}

      {view ? (
        <RunSummaryCard
          view={view}
          onCancel={running ? () => void cancel() : undefined}
          onContinue={view.stats.budgetExhausted ? () => void openSheet() : undefined}
        />
      ) : null}
    </section>
  );
}
