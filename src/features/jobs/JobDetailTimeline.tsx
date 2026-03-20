import { useEffect, useState } from "react";
import type { Job } from "../../lib/types";
import { listStatusHistory, saveApplicationPdf } from "../../lib/tauriApi";

type HistoryRow = { from_status: string | null; to_status: string; changed_at: string };

type Props = {
  selected?: Job;
  onSavedPdf: () => Promise<void>;
};

export function JobDetailTimeline({ selected, onSavedPdf }: Props) {
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

  if (!selected) return <section className="card"><h2>Detail Timeline</h2><p>Select a job.</p></section>;

  return (
    <section className="card">
      <h2>Detail Timeline</h2>
      <p><strong>{selected.company}</strong> - {selected.title ?? "Untitled"}</p>
      <p>Status: {selected.status}</p>
      <p>PDF: {selected.pdf_path ?? "None"}</p>
      <input type="file" accept="application/pdf" onChange={(e) => void onUploadPdf(e.target.files?.[0])} />
      <h3>History</h3>
      <ul>
        {history.map((h, idx) => (
          <li key={idx}>
            {h.from_status ?? "New"} {"->"} {h.to_status} ({new Date(h.changed_at).toLocaleString()})
          </li>
        ))}
      </ul>
    </section>
  );
}
