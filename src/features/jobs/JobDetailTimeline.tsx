import { memo, useEffect, useState } from "react";
import type { Job } from "../../lib/types";
import { listStatusHistory, saveApplicationPdf } from "../../lib/tauriApi";
import { en } from "../../i18n/en";

type HistoryRow = { from_status: string | null; to_status: string; changed_at: string };

type Props = {
  selected?: Job;
  onSavedPdf: () => void | Promise<void>;
};

export const JobDetailTimeline = memo(function JobDetailTimeline({ selected, onSavedPdf }: Props) {
  const [history, setHistory] = useState<HistoryRow[]>([]);

  useEffect(() => {
    if (!selected) return;
    void listStatusHistory(selected.id).then(setHistory);
  }, [selected]);

  async function onUploadPdf(file?: File | null) {
    if (!selected || !file) return;
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    await saveApplicationPdf(selected.id, file.name, bytes);
    await onSavedPdf();
  }

  if (!selected) {
    return (
      <section className="card">
        <h2>{en.detail.title}</h2>
        <p>{en.detail.selectJob}</p>
      </section>
    );
  }

  return (
    <section className="card">
      <h2>{en.detail.title}</h2>
      <p>
        <strong>{selected.company}</strong> - {selected.title ?? en.common.untitled}
      </p>
      <p>
        {en.detail.status} {selected.status}
      </p>
      <p>
        {en.detail.pdf} {selected.pdf_path ?? en.common.dash}
      </p>
      <input type="file" accept="application/pdf" onChange={(e) => void onUploadPdf(e.target.files?.[0])} />
      <h3>{en.detail.history}</h3>
      <ul>
        {history.map((h, idx) => (
          <li key={`${h.changed_at}-${idx}`}>
            {h.from_status ?? en.detail.newStatus} {"->"} {h.to_status} ({new Date(h.changed_at).toLocaleString()})
          </li>
        ))}
      </ul>
    </section>
  );
});
