/**
 * Settings → Mail scan (spec §11.3).
 *
 * Three things here are safety surfaces rather than conveniences:
 *
 * - Every folder shows its **resolved** path, so a shortcut pointing somewhere
 *   unexpected is visible before it is read (§6.4).
 * - Profiles show a size and a hash prefix, never their content (§6.5).
 * - `Delete all mail scan data` needs the word DELETE typed, and says in advance that
 *   Jobs are not touched.
 */

import { useCallback, useEffect, useState } from "react";
import { en } from "../../i18n/en";
import {
  mailScanDeleteAllData,
  mailScanDetectThunderbird,
  mailScanEstimate,
  mailScanProfileClear,
  mailScanProfileSetFromPath,
  mailScanProfileStatus,
  mailScanResolveSources,
  mailScanSettingsGet,
  mailScanSettingsSet,
  mailScanSidecarProbe,
  mailScanStart,
  mailScanTestSource,
  type MailScanSettingsPayload,
  type ProfileStatus,
  type ResolvedSource,
  type SidecarProbe,
  type SourceTestResult,
} from "./mailMatchApi";

const t = en.mailScanSettings;

type ProfileKind = "short" | "full";

function newSourceId(): string {
  return `src_${Date.now().toString(36)}`;
}

export function MailScanSettings() {
  const [settings, setSettings] = useState<MailScanSettingsPayload | null>(null);
  const [resolved, setResolved] = useState<ResolvedSource[]>([]);
  const [tests, setTests] = useState<Record<string, SourceTestResult>>({});
  const [profiles, setProfiles] = useState<Record<ProfileKind, ProfileStatus>>({
    short: { configured: false },
    full: { configured: false },
  });
  const [sidecar, setSidecar] = useState<SidecarProbe | null>(null);
  const [backlog, setBacklog] = useState(0);
  const [profilePaths, setProfilePaths] = useState<Record<ProfileKind, string>>({
    short: "",
    full: "",
  });
  const [deleteConfirm, setDeleteConfirm] = useState("");
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(
    () =>
      Promise.all([
        mailScanSettingsGet(),
        mailScanResolveSources(),
        mailScanProfileStatus("short"),
        mailScanProfileStatus("full"),
        mailScanSidecarProbe(),
      ])
        .then(([loaded, sources, short, full, probe]) => {
          setSettings(loaded);
          setResolved(sources);
          setProfiles({ short, full });
          setSidecar(probe);
          setError(null);
        })
        .catch((e: unknown) => setError(String(e))),
    [],
  );

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    void mailScanEstimate({})
      .then((e) => setBacklog(e.backlogUnderCutoff))
      .catch(() => setBacklog(0));
  }, [settings?.provider]);

  function patch(next: Partial<MailScanSettingsPayload>) {
    if (!settings) return;
    const merged = { ...settings, ...next };
    setSettings(merged);
    void mailScanSettingsSet(merged)
      .then(() => mailScanResolveSources())
      .then((sources) => {
        setResolved(sources);
        setNotice(t.saved);
      })
      .catch((e: unknown) => setError(String(e)));
  }

  /**
   * Point at a profile file by path.
   *
   * A native picker would be nicer, but it would mean adding the Tauri dialog plugin;
   * the security property is the same either way — the webview only ever handles the
   * *path*, and Rust reads the CV.
   */
  async function replaceProfile(kind: ProfileKind) {
    const path = profilePaths[kind].trim();
    if (!path) return;
    try {
      setProfiles({ ...profiles, [kind]: await mailScanProfileSetFromPath(kind, path) });
      setProfilePaths({ ...profilePaths, [kind]: "" });
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function clearProfile(kind: ProfileKind) {
    setProfiles({ ...profiles, [kind]: await mailScanProfileClear(kind) });
  }

  async function testSource(index: number) {
    if (!settings) return;
    const source = settings.sources[index];
    const result = await mailScanTestSource(source);
    setTests({ ...tests, [source.id]: result });
  }

  async function detectThunderbird() {
    const roots = await mailScanDetectThunderbird();
    setNotice(roots.length ? t.thunderbirdFound(roots.length) : t.thunderbirdNone);
    if (!roots.length || !settings) return;
    patch({
      sources: [
        ...settings.sources,
        { id: newSourceId(), label: "Thunderbird", path: roots[0], kind: "maildir" },
      ],
    });
  }

  async function rescoreBacklog() {
    if (!settings) return;
    try {
      await mailScanStart({
        sources: settings.sources,
        provider: settings.provider,
        cutoff: settings.cutoff,
        sinceDays: settings.sinceDays,
        maxCalls: settings.maxCalls,
        forceRescore: true,
      });
    } catch (e) {
      setError(String(e));
    }
  }

  async function deleteAll() {
    try {
      await mailScanDeleteAllData(deleteConfirm);
      setDeleteConfirm("");
      setBacklog(0);
      setNotice(t.deleteAllDone);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  if (!settings) return null;

  return (
    <section className="settingsSection mail-scan-settings">
      <h3 className="cardTitle">{t.heading}</h3>
      <p className="muted settingsHint">{t.hint}</p>

      {error ? (
        <p className="settingsError" role="alert">
          {error}
        </p>
      ) : null}
      {notice ? <p className="muted settingsHint">{notice}</p> : null}

      <h4 className="settingsSubTitle">{t.sourcesHeading}</h4>
      <p className="muted settingsHint">{t.sourcesHint}</p>
      {settings.sources.length === 0 ? <p className="muted">{t.sourceNone}</p> : null}
      <ul className="mail-scan-settings__sources">
        {settings.sources.map((source, index) => {
          const info = resolved.find((r) => r.id === source.id);
          const test = tests[source.id];
          return (
            <li key={source.id}>
              <label>
                {t.sourceLabel}
                <input
                  type="text"
                  value={source.label}
                  placeholder={t.sourceLabelPlaceholder}
                  onChange={(e) => {
                    const sources = [...settings.sources];
                    sources[index] = { ...source, label: e.target.value };
                    patch({ sources });
                  }}
                />
              </label>
              <label>
                {t.sourcePath}
                <input
                  type="text"
                  value={source.path}
                  placeholder={t.sourcePathPlaceholder}
                  onChange={(e) => {
                    const sources = [...settings.sources];
                    sources[index] = { ...source, path: e.target.value };
                    patch({ sources });
                  }}
                />
              </label>

              {info?.resolvedPath ? (
                <p className="muted settingsHint">{t.sourceResolved(info.resolvedPath)}</p>
              ) : null}
              {info?.redirected ? (
                <p className="settingsWarning">{t.sourceRedirected}</p>
              ) : null}
              {info?.error ? <p className="settingsError">{info.error}</p> : null}

              <div className="mail-scan-settings__source-actions">
                <button type="button" onClick={() => void testSource(index)}>
                  {t.sourceTest}
                </button>
                <button
                  type="button"
                  onClick={() =>
                    patch({ sources: settings.sources.filter((s) => s.id !== source.id) })
                  }
                >
                  {t.sourceRemove}
                </button>
              </div>

              {test ? (
                <p className="muted settingsHint">
                  {test.ok ? t.sourceTestResult(test.messageCount) : test.error}
                  {test.ok && test.earliest && test.latest
                    ? ` · ${t.sourceTestRange(test.earliest, test.latest)}`
                    : ""}
                </p>
              ) : null}
            </li>
          );
        })}
      </ul>
      <div className="mail-scan-settings__source-actions">
        <button
          type="button"
          onClick={() =>
            patch({
              sources: [
                ...settings.sources,
                { id: newSourceId(), label: "", path: "", kind: "mbox" },
              ],
            })
          }
        >
          {t.sourceAdd}
        </button>
        <button type="button" onClick={() => void detectThunderbird()}>
          {t.thunderbirdDetect}
        </button>
      </div>

      <h4 className="settingsSubTitle">{t.profilesHeading}</h4>
      <p className="muted settingsHint">{t.profilesHint}</p>
      {(["short", "full"] as const).map((kind) => {
        const status = profiles[kind];
        return (
          <div key={kind} className="mail-scan-settings__profile">
            <strong>{kind === "short" ? t.profileShort : t.profileFull}</strong>{" "}
            <span className="muted">
              {status.configured && status.sizeBytes !== undefined && status.hashPrefix
                ? t.profileConfigured(status.sizeBytes, status.hashPrefix)
                : t.profileMissing}
            </span>
            <input
              type="text"
              value={profilePaths[kind]}
              placeholder={t.profilePathPlaceholder}
              aria-label={t.profilePathLabel(kind === "short" ? t.profileShort : t.profileFull)}
              onChange={(e) => setProfilePaths({ ...profilePaths, [kind]: e.target.value })}
            />
            <button
              type="button"
              disabled={!profilePaths[kind].trim()}
              onClick={() => void replaceProfile(kind)}
            >
              {t.profileReplace}
            </button>
            {status.configured ? (
              <button type="button" onClick={() => void clearProfile(kind)}>
                {t.profileClear}
              </button>
            ) : null}
          </div>
        );
      })}

      <h4 className="settingsSubTitle">{t.scoringHeading}</h4>
      <label className="settingsRow">
        {t.cutoffLabel}
        <input
          type="number"
          min={0}
          max={10}
          value={settings.cutoff}
          onChange={(e) => patch({ cutoff: Number(e.target.value) })}
        />
      </label>
      <label className="settingsRow">
        {t.sinceLabel}
        <input
          type="number"
          min={1}
          value={settings.sinceDays}
          onChange={(e) => patch({ sinceDays: Number(e.target.value) })}
        />
      </label>
      <label className="settingsRow">
        {t.budgetLabel}
        <input
          type="number"
          min={1}
          value={settings.maxCalls}
          onChange={(e) => patch({ maxCalls: Number(e.target.value) })}
        />
      </label>
      <p className="muted settingsHint">{t.budgetHint}</p>

      <h4 className="settingsSubTitle">{t.sidecarHeading}</h4>
      {sidecar?.available ? (
        <p className="muted settingsHint">
          {t.sidecarMode(sidecar.description)} ·{" "}
          {sidecar.hashPinned ? t.sidecarPinned : t.sidecarUnpinned}
        </p>
      ) : (
        <p className="settingsError">{sidecar?.error ?? t.sidecarMissing}</p>
      )}

      <h4 className="settingsSubTitle">{t.dataHeading}</h4>
      <p className="muted settingsHint">
        {backlog > 0 ? t.rescoreBacklogHint(backlog) : t.rescoreBacklogNone}
      </p>
      <button type="button" disabled={backlog === 0} onClick={() => void rescoreBacklog()}>
        {t.rescoreBacklog}
      </button>

      <p className="muted settingsHint">{t.deleteAllHint}</p>
      <label className="settingsRow">
        {t.deleteAllConfirmLabel}
        <input
          type="text"
          value={deleteConfirm}
          onChange={(e) => setDeleteConfirm(e.target.value)}
        />
      </label>
      <button
        type="button"
        disabled={deleteConfirm.trim() !== "DELETE"}
        onClick={() => void deleteAll()}
      >
        {t.deleteAll}
      </button>
    </section>
  );
}
