/**
 * Route wrapper for the Mail Match Inbox (spec §11.1, ADR 0003) and Scan control (§11.2).
 *
 * Accepting a match creates the Job in place; this wrapper reloads the board's job
 * list afterwards and routes the notice's Open button to the Job's detail page.
 */

import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useJobTracker } from "../context/JobTrackerContext";
import { MailMatchInboxPanel } from "../features/mailMatch/MailMatchInboxPanel";
import { ScanControl } from "../features/mailMatch/ScanControl";
import {
  mailScanSettingsGet,
  type MailScanSettingsPayload,
} from "../features/mailMatch/mailMatchApi";
import { en } from "../i18n/en";

const t = en.mailMatch;

export function MailMatchPage() {
  const navigate = useNavigate();
  const { syncJobList } = useJobTracker();
  const [settings, setSettings] = useState<MailScanSettingsPayload | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);

  const loadSettings = useCallback(() => {
    void mailScanSettingsGet()
      .then((loaded) => {
        setSettings(loaded);
        setSettingsError(null);
      })
      .catch((e: unknown) => setSettingsError(String(e)));
  }, []);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  return (
    <div className="page page--mail-match mailMatchPage">
      <div className="mailMatchPageTop">
        <div>
          <h2 className="mailMatchTitle">{t.title}</h2>
          <p className="mailMatchSubtitle">{t.subtitle}</p>
        </div>
        {settings ? (
          <ScanControl
            sources={settings.sources}
            cutoff={settings.cutoff}
            onRunFinished={() => {
              setReloadToken((n) => n + 1);
              loadSettings();
            }}
          />
        ) : settingsError ? (
          <p className="mailMatchSettingsError" role="alert">
            {settingsError}
          </p>
        ) : null}
      </div>

      <MailMatchInboxPanel
        reloadToken={reloadToken}
        hidePageHeading
        onJobsChanged={() => void syncJobList()}
        onOpenJob={(jobId) => navigate(`/job/${jobId}`)}
      />
    </div>
  );
}
