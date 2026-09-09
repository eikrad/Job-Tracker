import { useEffect, useRef, useState } from "react";
import { ChevronUp, ChevronDown } from "lucide-react";
import { useJobTracker } from "../context/JobTrackerContext";
import { useTheme } from "../hooks/useTheme";
import type { ThemePreference } from "../lib/theme";
import { BOARD_VIEWS, type BoardView } from "../lib/jobs/boardViewPreference";
import { exportJobsAsCsv, exportJobsAsJson } from "../lib/export/exportBundle";
import { googleOauthGetClientId, googleOauthSetClientId, llmProviderOverrideGet, llmProviderOverrideSet, llmTestConnection } from "../lib/tauriApi";
import type { LlmProvider } from "../features/extraction/extractJobInfo";
import { SecretKeyField } from "./SecretKeyField";
import { en } from "../i18n/en";

function parseLlmProvider(value: string): LlmProvider {
  if (value === "mistral" || value === "gemini" || value === "scaleway_deepseek") return value;
  return "scaleway_deepseek";
}

const THEME_OPTIONS: { value: ThemePreference; label: string }[] = [
  { value: "system", label: en.app.themeSystem },
  { value: "light", label: en.app.themeLight },
  { value: "dark", label: en.app.themeDark },
];

const BOARD_VIEW_LABELS: Record<BoardView, string> = {
  kanban: en.nav.kanban,
  table: en.nav.table,
  calendar: en.nav.calendar,
};

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
    refreshManualGoogleTokenStatus,
    googleOauthConnected,
    refreshGoogleOauthStatus,
    connectGoogleCalendar,
    disconnectGoogleCalendar,
    defaultBoardView,
    setDefaultBoardView,
    statuses,
    renameStatus,
    moveStatus,
    onImportFile,
    backupFolder,
    setBackupFolder,
  } = useJobTracker();

  const { preference: themePreference, setPreference: setThemePreference } = useTheme();
  const [googleClientId, setGoogleClientId] = useState("");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [oauthBusy, setOauthBusy] = useState(false);
  const [testBusy, setTestBusy] = useState(false);
  const [testMessage, setTestMessage] = useState<string | null>(null);
  const [overrideBaseUrl, setOverrideBaseUrl] = useState("");
  const [overrideModelId, setOverrideModelId] = useState("");
  const [overrideBusy, setOverrideBusy] = useState(false);
  // This dialog is mounted for the whole session. Defer its body until first open so the
  // SecretKeyFields don't each fire a keyring round-trip on every app launch.
  const [hasOpened, setHasOpened] = useState(false);
  if (open && !hasOpened) setHasOpened(true);

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
      try {
        const ov = await llmProviderOverrideGet(llmProvider);
        setOverrideBaseUrl(ov.baseUrl ?? "");
        setOverrideModelId(ov.modelId ?? "");
      } catch {
        setOverrideBaseUrl("");
        setOverrideModelId("");
      }
    })();
  }, [open, refreshGoogleOauthStatus, llmProvider]);

  async function onTestConnection() {
    setTestBusy(true);
    setTestMessage(null);
    try {
      const res = await llmTestConnection(llmProvider);
      if (res.ok) {
        setTestMessage(en.app.llmTestConnectionOk(res.modelId ?? "?", res.detail ?? "OK"));
      } else {
        setTestMessage(en.app.llmTestConnectionFail(res.error ?? "Unknown error"));
      }
    } catch (e) {
      setTestMessage(en.app.llmTestConnectionFail(String(e)));
    } finally {
      setTestBusy(false);
    }
  }

  async function onSaveOverrides() {
    setOverrideBusy(true);
    try {
      await llmProviderOverrideSet(
        llmProvider,
        overrideBaseUrl.trim() || null,
        overrideModelId.trim() || null,
      );
      window.alert(en.app.llmOverrideSaved);
    } catch (e) {
      window.alert(String(e));
    } finally {
      setOverrideBusy(false);
    }
  }

  async function onResetOverrides() {
    setOverrideBusy(true);
    try {
      await llmProviderOverrideSet(llmProvider, null, null);
      setOverrideBaseUrl("");
      setOverrideModelId("");
      window.alert(en.app.llmOverrideSaved);
    } catch (e) {
      window.alert(String(e));
    } finally {
      setOverrideBusy(false);
    }
  }

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

  // Never opened this session: render the shell only, so none of the SecretKeyFields
  // below mount and hit the keyring.
  if (!hasOpened) {
    return <dialog ref={dialogRef} className="settingsDialog" />;
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
            <h3 className="cardTitle">{en.app.settingsSectionAppearance}</h3>
            <p className="muted settingsHint">{en.app.appearanceHint}</p>
            <div className="themeOptions" role="radiogroup" aria-label={en.app.themeLabel}>
              {THEME_OPTIONS.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  role="radio"
                  aria-checked={themePreference === option.value}
                  className={`btn btnSm ${
                    themePreference === option.value ? "btnPrimary" : "btnGhost"
                  }`}
                  onClick={() => setThemePreference(option.value)}
                >
                  {option.label}
                </button>
              ))}
            </div>
            <p className="muted settingsHint">{en.app.defaultBoardViewHint}</p>
            <div
              className="themeOptions"
              role="radiogroup"
              aria-label={en.app.defaultBoardViewLabel}
            >
              {BOARD_VIEWS.map((option) => (
                <button
                  key={option}
                  type="button"
                  role="radio"
                  aria-checked={defaultBoardView === option}
                  className={`btn btnSm ${
                    defaultBoardView === option ? "btnPrimary" : "btnGhost"
                  }`}
                  onClick={() => setDefaultBoardView(option)}
                >
                  {BOARD_VIEW_LABELS[option]}
                </button>
              ))}
            </div>
          </section>

          <section className="settingsSection">
            <h3 className="cardTitle">{en.app.settingsSectionIntegrations}</h3>
            <label>
              {en.app.aiExtractionProvider}
              <select
                value={llmProvider}
                onChange={(e) => setLlmProvider(parseLlmProvider(e.target.value))}
              >
                <option value="scaleway_deepseek">{en.app.aiExtractionProviderScaleway}</option>
                <option value="gemini">{en.app.aiExtractionProviderGemini}</option>
                <option value="mistral">{en.app.aiExtractionProviderMistral}</option>
              </select>
            </label>
            <div className="row settingsGoogleActions">
              <button
                type="button"
                className="btn btnSm btnPrimary"
                disabled={testBusy}
                onClick={() => void onTestConnection()}
              >
                {en.app.llmTestConnection}
              </button>
            </div>
            {testMessage ? <p className="muted settingsHint">{testMessage}</p> : null}
            <h4 className="settingsSubTitle">{en.app.llmOverrideHeading}</h4>
            <p className="muted settingsHint">{en.app.llmOverrideHint}</p>
            <label>
              {en.app.llmOverrideBaseUrl}
              <input
                value={overrideBaseUrl}
                onChange={(e) => setOverrideBaseUrl(e.target.value)}
                placeholder="https://api.scaleway.ai/v1"
                autoComplete="off"
                spellCheck={false}
              />
            </label>
            <label>
              {en.app.llmOverrideModelId}
              <input
                value={overrideModelId}
                onChange={(e) => setOverrideModelId(e.target.value)}
                placeholder="deepseek-v4-flash-0731"
                autoComplete="off"
                spellCheck={false}
              />
            </label>
            <div className="row settingsGoogleActions">
              <button
                type="button"
                className="btn btnSm btnPrimary"
                disabled={overrideBusy}
                onClick={() => void onSaveOverrides()}
              >
                {en.app.llmOverrideSave}
              </button>
              <button
                type="button"
                className="btn btnSm btnGhost"
                disabled={overrideBusy}
                onClick={() => void onResetOverrides()}
              >
                {en.app.llmOverrideReset}
              </button>
            </div>
            <SecretKeyField
              provider="scaleway"
              label={en.app.scalewayKey}
              placeholder={en.app.scalewayPlaceholder}
            />
            <SecretKeyField
              provider="gemini"
              label={en.app.geminiKey}
              placeholder={en.app.geminiPlaceholder}
            />
            <SecretKeyField
              provider="mistral"
              label={en.app.mistralKey}
              placeholder={en.app.mistralPlaceholder}
            />
            <SecretKeyField
              provider="serpapi"
              label={en.app.serpApiKey}
              placeholder={en.app.serpApiPlaceholder}
            />
            <SecretKeyField
              provider="brave"
              label={en.app.braveSearchApiKey}
              placeholder={en.app.braveSearchApiPlaceholder}
            />
            <p className="muted settingsHint">{en.app.jobSearchProviderHint}</p>

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
                  <SecretKeyField
                    provider="google_access_token"
                    label={en.app.googleToken}
                    placeholder={en.app.googlePlaceholder}
                    onStatusChange={() => void refreshManualGoogleTokenStatus()}
                  />
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
                  <ChevronUp size={14} />
                </button>
                <button
                  type="button"
                  className="btn btnGhost btnIcon"
                  aria-label={en.app.moveColumnDown}
                  onClick={() => moveStatus(index, 1)}
                >
                  <ChevronDown size={14} />
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
