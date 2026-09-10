/**
 * Route wrapper for the Mail Match Inbox (spec §11.1, ADR 0003) and Scan control (§11.2).
 *
 * Accepting a match navigates to the job form with the draft prefilled, so the last
 * step before a Job exists is always a form the user submits.
 */

import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import type { NewJob } from "../lib/types";
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
            provider={settings.provider}
            cutoff={settings.cutoff}
            sinceDays={settings.sinceDays}
            maxCalls={settings.maxCalls}
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
        onAcceptDraft={(inboxId, draft: Partial<NewJob>) => {
          navigate("/jobs/new", { state: { draft, mailMatchInboxId: inboxId } });
        }}
      />
    </div>
  );
}
