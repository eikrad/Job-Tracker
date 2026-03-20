import { memo, useState } from "react";
import { effectiveStatuses } from "../../lib/statusUtils";
import type { NewJob } from "../../lib/types";
import { DEFAULT_STATUSES } from "../../lib/types";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { en } from "../../i18n/en";

type Props = {
  statuses: string[];
  onSubmit: (payload: NewJob) => Promise<boolean>;
  onExtract: (rawText: string) => Promise<ExtractJobInfoResult>;
  hideTitle?: boolean;
  onSubmitted?: () => void;
};

export const JobForm = memo(function JobForm({
  statuses,
  onSubmit,
  onExtract,
  hideTitle = false,
  onSubmitted,
}: Props) {
  const lanes = effectiveStatuses(statuses);
  const [form, setForm] = useState<NewJob>(() => ({
    company: "",
    status: lanes[0] ?? DEFAULT_STATUSES[0],
  }));
  const [error, setError] = useState("");
  const [extractError, setExtractError] = useState("");
  const [suggestion, setSuggestion] = useState<Partial<NewJob> | null>(null);

  const update = (patch: Partial<NewJob>) => setForm((v) => ({ ...v, ...patch }));

  async function submit() {
    if (!form.company.trim()) return setError(en.jobForm.companyRequired);
    if (!form.status) return setError(en.jobForm.statusRequired);
    setError("");
    const saved = await onSubmit(form);
    if (!saved) return;
    const nextLanes = effectiveStatuses(statuses);
    setForm({ company: "", status: nextLanes[0] ?? DEFAULT_STATUSES[0] });
    onSubmitted?.();
  }

  async function extract() {
    setExtractError("");
    const result = await onExtract(form.raw_text ?? "");
    if (!result.ok) {
      setExtractError(result.error);
      return;
    }
    setSuggestion(result.partial);
  }

  function applySuggestion() {
    if (!suggestion) return;
    setForm((v) => ({ ...v, ...suggestion }));
    setSuggestion(null);
  }

  return (
    <section className="card">
      {!hideTitle && <h2>{en.jobForm.sectionTitle}</h2>}
      <div className="grid">
        <input
          placeholder={en.jobForm.companyPh}
          value={form.company}
          onChange={(e) => update({ company: e.target.value })}
        />
        <input
          placeholder={en.jobForm.titlePh}
          value={form.title ?? ""}
          onChange={(e) => update({ title: e.target.value })}
        />
        <select value={form.status} onChange={(e) => update({ status: e.target.value })}>
          {lanes.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <input type="date" value={form.deadline ?? ""} onChange={(e) => update({ deadline: e.target.value })} />
        <input
          placeholder={en.jobForm.jobUrl}
          value={form.url ?? ""}
          onChange={(e) => update({ url: e.target.value })}
        />
        <input
          placeholder={en.jobForm.tagsPh}
          value={form.tags ?? ""}
          onChange={(e) => update({ tags: e.target.value })}
        />
      </div>
      <div className="fieldFull">
        <textarea
          rows={5}
          placeholder={en.jobForm.pasteAd}
          value={form.raw_text ?? ""}
          onChange={(e) => update({ raw_text: e.target.value })}
        />
      </div>
      <div className="fieldFull">
        <textarea
          rows={3}
          placeholder={en.jobForm.notesPh}
          value={form.notes ?? ""}
          onChange={(e) => update({ notes: e.target.value })}
        />
      </div>
      <div className="row formActions">
        <button type="button" className="btn btnGhost" onClick={() => void extract()}>
          {en.jobForm.extractWithAi}
        </button>
        <button type="button" className="btn btnPrimary" onClick={() => void submit()}>
          {en.jobForm.save}
        </button>
      </div>
      {extractError && <p className="error extractError">{extractError}</p>}
      {suggestion && Object.keys(suggestion).length > 0 && (
        <div className="card card--nested">
          <p className="muted suggestionLead">{en.jobForm.suggestionReady}</p>
          <pre>{JSON.stringify(suggestion, null, 2)}</pre>
          <button type="button" className="btn btnPrimary" onClick={applySuggestion}>
            {en.jobForm.applySuggestion}
          </button>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  );
});
