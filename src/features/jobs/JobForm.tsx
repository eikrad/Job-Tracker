import { memo, useState } from "react";
import { effectiveStatuses } from "../../lib/statusUtils";
import type { NewJob } from "../../lib/types";
import { DEFAULT_STATUSES } from "../../lib/types";
import { en } from "../../i18n/en";

type Props = {
  statuses: string[];
  onSubmit: (payload: NewJob) => Promise<void>;
  onExtract: (rawText: string) => Promise<Partial<NewJob>>;
};

export const JobForm = memo(function JobForm({ statuses, onSubmit, onExtract }: Props) {
  const lanes = effectiveStatuses(statuses);
  const [form, setForm] = useState<NewJob>(() => ({
    company: "",
    status: lanes[0] ?? DEFAULT_STATUSES[0],
  }));
  const [error, setError] = useState("");
  const [suggestion, setSuggestion] = useState<Partial<NewJob> | null>(null);

  const update = (patch: Partial<NewJob>) => setForm((v) => ({ ...v, ...patch }));

  async function submit() {
    if (!form.company.trim()) return setError(en.jobForm.companyRequired);
    if (!form.status) return setError(en.jobForm.statusRequired);
    setError("");
    await onSubmit(form);
    const nextLanes = effectiveStatuses(statuses);
    setForm({ company: "", status: nextLanes[0] ?? DEFAULT_STATUSES[0] });
  }

  async function extract() {
    const next = await onExtract(form.raw_text ?? "");
    setSuggestion(next);
  }

  function applySuggestion() {
    if (!suggestion) return;
    setForm((v) => ({ ...v, ...suggestion }));
    setSuggestion(null);
  }

  return (
    <section className="card">
      <h2>{en.jobForm.sectionTitle}</h2>
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
      <textarea
        rows={5}
        placeholder={en.jobForm.pasteAd}
        value={form.raw_text ?? ""}
        onChange={(e) => update({ raw_text: e.target.value })}
      />
      <textarea
        rows={3}
        placeholder={en.jobForm.notesPh}
        value={form.notes ?? ""}
        onChange={(e) => update({ notes: e.target.value })}
      />
      <div className="row">
        <button type="button" onClick={() => void extract()}>
          {en.jobForm.extractGemini}
        </button>
        <button type="button" onClick={() => void submit()}>
          {en.jobForm.save}
        </button>
      </div>
      {suggestion && (
        <div className="card">
          <p>{en.jobForm.suggestionReady}</p>
          <pre>{JSON.stringify(suggestion, null, 2)}</pre>
          <button type="button" onClick={applySuggestion}>
            {en.jobForm.applySuggestion}
          </button>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  );
});
