import { useEffect, useRef, useState } from "react";
import { useJobTracker } from "../context/JobTrackerContext";
import { exportJobsAsCsv, exportJobsAsJson } from "../lib/export/exportBundle";
import { googleOauthGetClientId, googleOauthSetClientId } from "../lib/tauriApi";
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
    googleOauthConnected,
    refreshGoogleOauthStatus,
    connectGoogleCalendar,
    disconnectGoogleCalendar,
    statuses,
    renameStatus,
    moveStatus,
    onImportFile,
    backupFolder,
    setBackupFolder,
  } = useJobTracker();

  const [googleClientId, setGoogleClientId] = useState("");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [oauthBusy, setOauthBusy] = useState(false);

  useEffect(() => {
    const el = dialogRef.current;
    if (!el) return;
    if (open) {
      el.showModal();
    } else {
      el.close();
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    void (async () => {
      try {
        const id = await googleOauthGetClientId();
        setGoogleClientId(id);
      } catch {
        setGoogleClientId("");
      }
      await refreshGoogleOauthStatus();
    })();
  }, [open, refreshGoogleOauthStatus]);

  async function saveGoogleClientId() {
    try {
      await googleOauthSetClientId(googleClientId.trim());
      window.alert(en.app.googleClientIdSaved);
    } catch (e) {
      window.alert(String(e));
    }
  }

  async function onConnectGoogle() {
    if (!googleClientId.trim()) {
      window.alert(en.app.googleClientIdRequired);
      return;
    }
    setOauthBusy(true);
    try {
      await googleOauthSetClientId(googleClientId.trim());
      await connectGoogleCalendar();
      window.alert(en.app.googleConnectSuccess);
    } catch (e) {
      window.alert(String(e));
    } finally {
      setOauthBusy(false);
      await refreshGoogleOauthStatus();
    }
  }

  async function onDisconnectGoogle() {
    try {
      await disconnectGoogleCalendar();
      await refreshGoogleOauthStatus();
    } catch (e) {
      window.alert(String(e));
    }
  }

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

            <div className="settingsGoogleBlock">
              <h4 className="settingsSubTitle">{en.app.googleCalendarHeading}</h4>
              <p className="muted settingsHint">{en.app.googleOAuthIntro}</p>
              <label>
                {en.app.googleOAuthClientId}
                <input
                  value={googleClientId}
                  onChange={(e) => setGoogleClientId(e.target.value)}
                  placeholder={en.app.googleOAuthClientIdPlaceholder}
                  autoComplete="off"
                  spellCheck={false}
                />
              </label>
              <div className="row settingsGoogleActions">
                <button type="button" className="btn btnGhost btnSm" onClick={() => void saveGoogleClientId()}>
                  {en.app.googleSaveClientId}
                </button>
              </div>
              <p className="muted settingsHint">
                {en.app.googleOAuthStatus(googleOauthConnected ? "yes" : "no")}
              </p>
              <div className="row settingsGoogleActions">
                <button
                  type="button"
                  className="btn btnPrimary btnSm"
                  disabled={oauthBusy}
                  onClick={() => void onConnectGoogle()}
                >
                  {en.app.googleConnect}
                </button>
                <button
                  type="button"
                  className="btn btnGhost btnSm"
                  disabled={!googleOauthConnected}
                  onClick={() => void onDisconnectGoogle()}
                >
                  {en.app.googleDisconnect}
                </button>
              </div>
            </div>

            <div className="settingsAdvanced">
              <button
                type="button"
                className="btn btnGhost btnSm settingsAdvancedToggle"
                aria-expanded={advancedOpen}
                onClick={() => setAdvancedOpen((v) => !v)}
              >
                {advancedOpen ? en.app.googleAdvancedHide : en.app.googleAdvancedShow}
              </button>
              {advancedOpen && (
                <div className="settingsAdvancedBody">
                  <p className="muted settingsHint">{en.app.googleAdvancedHelp}</p>
                  <label>
                    {en.app.googleToken}
                    <input
                      value={googleAccessToken}
                      onChange={(e) => setGoogleAccessToken(e.target.value)}
                      placeholder={en.app.googlePlaceholder}
                      autoComplete="off"
                    />
                  </label>
                </div>
              )}
            </div>
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

            <h4 className="settingsSubTitle">Backup</h4>
            <label>
              Backup folder path
              <input
                type="text"
                value={backupFolder}
                onChange={(e) => setBackupFolder(e.target.value)}
                placeholder="~/Jottacloud"
                autoComplete="off"
                spellCheck={false}
              />
            </label>
            <p className="muted settingsHint">Backup is automatic after every change.</p>
          </section>
        </div>
      </div>
    </dialog>
  );
}
