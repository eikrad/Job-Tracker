import { useEffect, useState } from "react";
import { llmKeyClear, llmKeySet, llmKeyStatus, type LlmKeyStatus } from "../lib/tauriApi";
import { en } from "../i18n/en";

type Props = {
  provider: string;
  label: string;
  placeholder: string;
  /** Called after a successful save or remove (status may have changed). */
  onStatusChange?: () => void;
};

/** Write-only secret field: never binds a stored key into the input value. */
export function SecretKeyField({ provider, label, placeholder, onStatusChange }: Props) {
  const [status, setStatus] = useState<LlmKeyStatus | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const next = await llmKeyStatus(provider);
        if (!cancelled) setStatus(next);
      } catch {
        if (!cancelled) setStatus({ configured: false, backend: "unknown" });
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [provider]);

  async function refresh() {
    try {
      setStatus(await llmKeyStatus(provider));
    } catch {
      setStatus({ configured: false, backend: "unknown" });
    }
  }

  /** Run a store mutation, then re-read status so the UI reflects what actually stuck. */
  async function runStoreAction(action: () => Promise<void>, successMessage: string) {
    setBusy(true);
    setMessage(null);
    try {
      await action();
      setDraft("");
      await refresh();
      onStatusChange?.();
      setMessage(successMessage);
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onReplace() {
    const key = draft.trim();
    if (!key) {
      setMessage(en.app.secretKeyEmpty);
      return;
    }
    await runStoreAction(() => llmKeySet(provider, key), en.app.secretKeySaved);
  }

  async function onRemove() {
    if (!window.confirm(en.app.secretKeyRemoveConfirm)) return;
    await runStoreAction(() => llmKeyClear(provider), en.app.secretKeyRemoved);
  }

  const configured = status?.configured ?? false;
  const backend = status?.backend ?? "";

  return (
    <div className="secretKeyField">
      <label>
        {label}
        <input
          type="password"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          placeholder={configured ? en.app.secretKeyReplacePlaceholder : placeholder}
          autoComplete="off"
          spellCheck={false}
          disabled={busy}
        />
      </label>
      <p className="muted settingsHint">
        {configured
          ? en.app.secretKeyConfigured(backend)
          : en.app.secretKeyNotConfigured}
        {backend === "file" ? ` ${en.app.secretKeyFileBackendHint}` : ""}
      </p>
      <div className="themeOptions">
        <button type="button" className="btn btnSm btnPrimary" disabled={busy} onClick={() => void onReplace()}>
          {configured ? en.app.secretKeyReplace : en.app.secretKeySave}
        </button>
        {configured ? (
          <button type="button" className="btn btnSm btnGhost" disabled={busy} onClick={() => void onRemove()}>
            {en.app.secretKeyRemove}
          </button>
        ) : null}
      </div>
      {message ? <p className="muted settingsHint">{message}</p> : null}
    </div>
  );
}
