import { useState } from "react";
import type { NewJob } from "../../lib/types";
import { DEFAULT_STATUSES } from "../../lib/types";

type Props = {
  statuses: string[];
  onSubmit: (payload: NewJob) => Promise<void>;
  onExtract: (rawText: string) => Promise<Partial<NewJob>>;
};

export function JobForm({ statuses, onSubmit, onExtract }: Props) {
  const [form, setForm] = useState<NewJob>({ company: "", status: DEFAULT_STATUSES[0] });
  const [error, setError] = useState("");
  const [suggestion, setSuggestion] = useState<Partial<NewJob> | null>(null);

  const update = (patch: Partial<NewJob>) => setForm((v) => ({ ...v, ...patch }));

  async function submit() {
    if (!form.company.trim()) return setError("Company is required.");
    if (!form.status) return setError("Status is required.");
    setError("");
    await onSubmit(form);
    setForm({ company: "", status: DEFAULT_STATUSES[0] });
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
      <h2>Add Job</h2>
      <div className="grid">
        <input placeholder="Company *" value={form.company} onChange={(e) => update({ company: e.target.value })} />
        <input placeholder="Title" value={form.title ?? ""} onChange={(e) => update({ title: e.target.value })} />
        <select value={form.status} onChange={(e) => update({ status: e.target.value })}>
          {(statuses.length ? statuses : DEFAULT_STATUSES).map((s) => <option key={s}>{s}</option>)}
        </select>
        <input type="date" value={form.deadline ?? ""} onChange={(e) => update({ deadline: e.target.value })} />
        <input placeholder="Job URL" value={form.url ?? ""} onChange={(e) => update({ url: e.target.value })} />
        <input placeholder="Tags (comma)" value={form.tags ?? ""} onChange={(e) => update({ tags: e.target.value })} />
      </div>
      <textarea
        rows={5}
        placeholder="Paste job ad text here"
        value={form.raw_text ?? ""}
        onChange={(e) => update({ raw_text: e.target.value })}
      />
      <textarea
        rows={3}
        placeholder="Notes"
        value={form.notes ?? ""}
        onChange={(e) => update({ notes: e.target.value })}
      />
      <div className="row">
        <button onClick={extract}>Extract with Gemini</button>
        <button onClick={submit}>Save</button>
      </div>
      {suggestion && (
        <div className="card">
          <p>Extraction suggestion ready. Review then apply.</p>
          <pre>{JSON.stringify(suggestion, null, 2)}</pre>
          <button onClick={applySuggestion}>Apply suggestion</button>
        </div>
      )}
      {error && <p className="error">{error}</p>}
    </section>
  );
}
