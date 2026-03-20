import { useEffect, useRef } from "react";
import { useJobTracker } from "../context/JobTrackerContext";
import { exportJobsAsCsv, exportJobsAsJson } from "../lib/export/exportBundle";
import { en } from "../i18n/en";

type Props = {
  open: boolean;
  onClose: () => void;
};

export function SettingsModal({ open, onClose }: Props) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const {
    jobs,
    llmProvider,
    setLlmProvider,
    geminiApiKey,
    setGeminiApiKey,
    mistralApiKey,
    setMistralApiKey,
    googleAccessToken,
    setGoogleAccessToken,
    statuses,
    renameStatus,
    moveStatus,
    onImportFile,
  } = useJobTracker();

  useEffect(() => {
    const el = dialogRef.current;
    if (!el) return;
    if (open) {
      el.showModal();
    } else {
      el.close();
    }
  }, [open]);

  return (
    <dialog
      ref={dialogRef}
      className="settingsDialog"
      aria-labelledby="settings-title"
      onCancel={(e) => {
        e.preventDefault();
        onClose();
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="settingsDialogPanel" onClick={(e) => e.stopPropagation()}>
        <div className="settingsDialogHeader">
          <h2 id="settings-title" className="settingsDialogTitle">
            {en.app.settingsTitle}
          </h2>
          <button type="button" className="btn btnGhost settingsDialogClose" onClick={onClose}>
            {en.app.settingsClose}
          </button>
        </div>

        <div className="settingsDialogBody">
          <section className="settingsSection">
            <h3 className="cardTitle">{en.app.settingsSectionIntegrations}</h3>
            <label>
              {en.app.aiExtractionProvider}
              <select
                value={llmProvider}
                onChange={(e) => setLlmProvider(e.target.value === "mistral" ? "mistral" : "gemini")}
              >
                <option value="gemini">{en.app.aiExtractionProviderGemini}</option>
                <option value="mistral">{en.app.aiExtractionProviderMistral}</option>
              </select>
            </label>
            <label>
              {llmProvider === "gemini" ? en.app.geminiKey : en.app.mistralKey}
              <input
                value={llmProvider === "gemini" ? geminiApiKey : mistralApiKey}
                onChange={(e) =>
                  llmProvider === "gemini"
                    ? setGeminiApiKey(e.target.value)
                    : setMistralApiKey(e.target.value)
                }
                placeholder={
                  llmProvider === "gemini" ? en.app.geminiPlaceholder : en.app.mistralPlaceholder
                }
                autoComplete="off"
              />
            </label>
            <label>
              {en.app.googleToken}
              <input
                value={googleAccessToken}
                onChange={(e) => setGoogleAccessToken(e.target.value)}
                placeholder={en.app.googlePlaceholder}
                autoComplete="off"
              />
            </label>
          </section>

          <section className="settingsSection">
            <h3 className="cardTitle">{en.app.settingsSectionPipeline}</h3>
            <p className="muted settingsHint">{en.app.statusColumns}</p>
            {statuses.map((status, index) => (
              <div className="statusRow" key={`${status}-${index}`}>
                <input
                  type="text"
                  value={status}
                  onChange={(e) => renameStatus(index, e.target.value)}
                  aria-label={`${en.app.statusColumns}: ${status}`}
                />
                <button
                  type="button"
                  className="btn btnGhost btnIcon"
                  aria-label={en.app.moveColumnUp}
                  onClick={() => moveStatus(index, -1)}
                >
                  ↑
                </button>
                <button
                  type="button"
                  className="btn btnGhost btnIcon"
                  aria-label={en.app.moveColumnDown}
                  onClick={() => moveStatus(index, 1)}
                >
                  ↓
                </button>
              </div>
            ))}
          </section>

          <section className="settingsSection">
            <h3 className="cardTitle">{en.app.settingsSectionData}</h3>
            <p className="muted settingsHint">{en.app.settingsDataHint}</p>
            <div className="row settingsDataActions">
              <button type="button" className="btn btnGhost" onClick={() => exportJobsAsJson(jobs)}>
                {en.nav.exportJson}
              </button>
              <button type="button" className="btn btnGhost" onClick={() => exportJobsAsCsv(jobs)}>
                {en.nav.exportCsv}
              </button>
              <label className="btn btnGhost fileImport">
                <span>{en.nav.importLabel}</span>
                <input
                  type="file"
                  className="visuallyHidden"
                  accept=".json,.csv,application/json,text/csv"
                  onChange={(e) => void onImportFile(e.target.files?.[0])}
                />
              </label>
            </div>
          </section>
        </div>
      </div>
    </dialog>
  );
}
