import { useMemo, useState } from "react";
import { fetchJobSearchResultPageText } from "../../lib/tauriApi";
import type { NewJob } from "../../lib/types";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { en } from "../../i18n/en";
import { createBaseCaptureDraft, normalizeCaptureUrl } from "./urlCapture";

type CaptureState = "idle" | "fetching" | "extracted" | "failed" | "saved";

type Props = {
  statuses: string[];
  onExtract: (rawText: string) => Promise<ExtractJobInfoResult>;
  onSubmit: (payload: NewJob) => Promise<boolean>;
  autoFocusInput?: boolean;
};

function mergeDraft(base: NewJob, partial: Partial<NewJob>): NewJob {
  const next = { ...base };
  for (const [key, value] of Object.entries(partial)) {
    if (value == null) continue;
    if (typeof value === "string" && !value.trim()) continue;
    (next as Record<string, unknown>)[key] = value;
  }
  return next;
}

export function UrlCaptureCard({ statuses, onExtract, onSubmit, autoFocusInput = false }: Props) {
  const defaultStatus = useMemo(() => statuses[0] ?? "Interesting", [statuses]);
  const [urlInput, setUrlInput] = useState("");
  const [state, setState] = useState<CaptureState>("idle");
  const [error, setError] = useState("");
  const [fetchWarning, setFetchWarning] = useState("");
  const [draft, setDraft] = useState<NewJob | null>(null);

  async function handleCapture() {
    setError("");
    setFetchWarning("");
    const normalizedUrl = normalizeCaptureUrl(urlInput);
    if (!normalizedUrl) {
      setState("failed");
      setDraft(null);
      setError(en.capture.invalidUrl);
      return;
    }

    setState("fetching");
    let rawText: string | undefined;
    try {
      const fetched = await fetchJobSearchResultPageText(normalizedUrl);
      if (fetched.trim()) rawText = fetched.trim();
    } catch {
      setFetchWarning(en.capture.fetchFallbackHint);
    }

    let nextDraft = createBaseCaptureDraft(normalizedUrl, defaultStatus);
    if (rawText) {
      nextDraft = { ...nextDraft, raw_text: rawText };
      const extracted = await onExtract(rawText);
      if (extracted.ok) {
        nextDraft = mergeDraft(nextDraft, extracted.partial);
      } else {
        setFetchWarning((prev) => prev || extracted.error || en.capture.extractFallbackHint);
      }
    }

    setDraft(nextDraft);
    setState("extracted");
  }

  async function handleSave() {
    if (!draft) return;
    if (!draft.company.trim()) {
      setError(en.capture.companyRequired);
      return;
    }
    if (!draft.url?.trim()) {
      setError(en.capture.urlRequired);
      return;
    }
    setError("");
    const ok = await onSubmit(draft);
    if (!ok) return;
    setState("saved");
    setUrlInput("");
    setDraft(null);
    setFetchWarning("");
  }

  return (
    <section className="card captureCard">
      <h2>{en.capture.title}</h2>
      <p className="muted">{en.capture.subtitle}</p>
      <div className="row captureInputRow">
        <input
          value={urlInput}
          onChange={(e) => setUrlInput(e.target.value)}
          placeholder={en.capture.urlPlaceholder}
          aria-label={en.capture.urlAriaLabel}
          autoFocus={autoFocusInput}
        />
        <button
          type="button"
          className="btn btnPrimary"
          onClick={() => void handleCapture()}
          disabled={state === "fetching"}
        >
          {state === "fetching" ? en.capture.capturing : en.capture.capture}
        </button>
      </div>

      {fetchWarning && <p className="muted">{fetchWarning}</p>}
      {error && <p className="error">{error}</p>}
      {state === "saved" && <p className="muted">{en.capture.saved}</p>}

      {draft && (
        <div className="card card--nested">
          <p className="cardTitle">{en.capture.previewTitle}</p>
          <div className="grid">
            <label className="fieldLabelStack">
              <span className="fieldLabelText">{en.capture.companyLabel}</span>
              <input
                value={draft.company}
                onChange={(e) => setDraft((prev) => (prev ? { ...prev, company: e.target.value } : prev))}
              />
            </label>
            <label className="fieldLabelStack">
              <span className="fieldLabelText">{en.capture.titleLabel}</span>
              <input
                value={draft.title ?? ""}
                onChange={(e) => setDraft((prev) => (prev ? { ...prev, title: e.target.value } : prev))}
              />
            </label>
            <label className="fieldLabelStack">
              <span className="fieldLabelText">{en.capture.locationLabel}</span>
              <input
                value={draft.workplace_city ?? ""}
                onChange={(e) => setDraft((prev) => (prev ? { ...prev, workplace_city: e.target.value } : prev))}
              />
            </label>
            <label className="fieldLabelStack">
              <span className="fieldLabelText">{en.capture.sourceLabel}</span>
              <input
                value={draft.source ?? ""}
                onChange={(e) => setDraft((prev) => (prev ? { ...prev, source: e.target.value } : prev))}
              />
            </label>
          </div>
          <div className="row">
            <button type="button" className="btn btnPrimary" onClick={() => void handleSave()}>
              {en.capture.saveAsInteresting}
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
