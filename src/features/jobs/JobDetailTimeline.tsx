import { memo, useEffect, useState } from "react";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { JobForm } from "./JobForm";
import type { Job, NewJob } from "../../lib/types";
import { listStatusHistory, saveApplicationPdf } from "../../lib/tauriApi";
import { en } from "../../i18n/en";

type HistoryRow = { from_status: string | null; to_status: string; changed_at: string };

type Props = {
  selected?: Job;
  onSavedPdf: () => void | Promise<void>;
  onDeleteJob: (jobId: number) => Promise<void>;
  statuses: string[];
  onExtract: (rawText: string) => Promise<ExtractJobInfoResult>;
  onUpdateJob: (jobId: number, payload: NewJob) => Promise<boolean>;
};

export const JobDetailTimeline = memo(function JobDetailTimeline({
  selected,
  onSavedPdf,
  onDeleteJob,
  statuses,
  onExtract,
  onUpdateJob,
}: Props) {
  const [history, setHistory] = useState<HistoryRow[]>([]);
  const [editing, setEditing] = useState(false);

  useEffect(() => {
    if (!selected) return;
    void listStatusHistory(selected.id).then(setHistory);
  }, [selected]);

  useEffect(() => {
    // Selecting another job must exit edit mode (form content is job-specific).
    // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional reset on selected.id
    setEditing(false);
  }, [selected?.id]);

  async function onUploadPdf(file?: File | null) {
    if (!selected || !file) return;
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    await saveApplicationPdf(selected.id, file.name, bytes);
    await onSavedPdf();
  }

  async function onDelete() {
    if (!selected) return;
    if (!window.confirm(en.alerts.deleteJobConfirm)) return;
    try {
      await onDeleteJob(selected.id);
    } catch (e) {
      window.alert(en.alerts.deleteJobFailed(e instanceof Error ? e.message : String(e)));
    }
  }

  if (!selected) {
    return (
      <section className="card">
        <h2>{en.detail.title}</h2>
        <p className="muted">{en.detail.selectJob}</p>
      </section>
    );
  }

  if (editing) {
    return (
      <section className="card">
        <h2>{en.detail.title}</h2>
        <p className="detailMeta">
          <strong>{selected.company}</strong> — {selected.title ?? en.common.untitled}
        </p>
        <h3 className="cardTitle">{en.jobForm.editSectionTitle}</h3>
        <JobForm
          key={selected.id}
          statuses={statuses}
          editingJob={selected}
          onSubmit={async () => false}
          onUpdateJob={onUpdateJob}
          onEditClose={() => setEditing(false)}
          onExtract={onExtract}
          hideTitle
        />
      </section>
    );
  }

  return (
    <section className="card">
      <h2>{en.detail.title}</h2>
      <p className="detailMeta">
        <strong>{selected.company}</strong> — {selected.title ?? en.common.untitled}
      </p>
      <p className="detailMeta">
        {en.detail.status} <span>{selected.status}</span>
      </p>
      {(selected.deadline || selected.interview_date || selected.start_date) && (
        <p className="detailMeta detailDates">
          {selected.deadline && (
            <>
              {en.detail.deadlineShort}: <span>{selected.deadline}</span>
            </>
          )}
          {selected.interview_date && (
            <>
              {selected.deadline ? " · " : null}
              {en.detail.interviewShort}: <span>{selected.interview_date}</span>
            </>
          )}
          {selected.start_date && (
            <>
              {selected.deadline || selected.interview_date ? " · " : null}
              {en.detail.startShort}: <span>{selected.start_date}</span>
            </>
          )}
        </p>
      )}
      <p className="detailMeta">
        {en.detail.pdf} <span>{selected.pdf_path ?? en.common.dash}</span>
      </p>
      <div className="detailActions row">
        <button type="button" className="btn btnGhost btnSm" onClick={() => setEditing(true)}>
          {en.detail.editJob}
        </button>
        <label className="btn btnGhost btnSm fileImport fileImportBlock">
          <span>{en.detail.uploadPdf}</span>
          <input
            type="file"
            accept="application/pdf"
            className="visuallyHidden"
            onChange={(e) => void onUploadPdf(e.target.files?.[0])}
          />
        </label>
        <button type="button" className="btn btnDanger btnSm" onClick={() => void onDelete()}>
          {en.detail.deleteJob}
        </button>
      </div>
      <p className="cardTitle">{en.detail.history}</p>
      <ul className="listPlain historyList">
        {history.map((h, idx) => (
          <li key={`${h.changed_at}-${idx}`}>
            {h.from_status ?? en.detail.newStatus} {"->"} {h.to_status} ({new Date(h.changed_at).toLocaleString()})
          </li>
        ))}
      </ul>
    </section>
  );
});
