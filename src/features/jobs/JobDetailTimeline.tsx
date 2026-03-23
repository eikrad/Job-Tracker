import { memo, useEffect, useState } from "react";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { JobForm } from "./JobForm";
import type { DocType, Job, JobDocument, NewJob } from "../../lib/types";
import { deleteJobDocument, listJobDocuments, listStatusHistory, saveJobDocument } from "../../lib/tauriApi";
import { en } from "../../i18n/en";

const DOC_TYPES: { value: DocType; label: string }[] = [
  { value: "cv", label: en.detail.docTypeCv },
  { value: "cover_letter", label: en.detail.docTypeCoverLetter },
  { value: "other", label: en.detail.docTypeOther },
];

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
  const [documents, setDocuments] = useState<JobDocument[]>([]);
  const [uploadDocType, setUploadDocType] = useState<DocType>("cv");

  useEffect(() => {
    if (!selected) return;
    void listStatusHistory(selected.id).then(setHistory);
    void listJobDocuments(selected.id).then(setDocuments);
  }, [selected]);

  useEffect(() => {
    // Selecting another job must exit edit mode (form content is job-specific).
    // eslint-disable-next-line react-hooks/set-state-in-effect -- intentional reset on selected.id
    setEditing(false);
  }, [selected?.id]);

  async function onUploadDocument(file?: File | null) {
    if (!selected || !file) return;
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    const doc = await saveJobDocument(selected.id, uploadDocType, file.name, bytes);
    setDocuments((prev) => [...prev, doc]);
    await onSavedPdf();
  }

  async function onDeleteDocument(docId: number) {
    await deleteJobDocument(docId);
    setDocuments((prev) => prev.filter((d) => d.id !== docId));
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
      <div className="detailActions row">
        <button type="button" className="btn btnGhost btnSm" onClick={() => setEditing(true)}>
          {en.detail.editJob}
        </button>
        <button type="button" className="btn btnDanger btnSm" onClick={() => void onDelete()}>
          {en.detail.deleteJob}
        </button>
      </div>
      <p className="cardTitle">{en.detail.documents}</p>
      {documents.length === 0 ? (
        <p className="muted">{en.detail.noDocuments}</p>
      ) : (
        <ul className="listPlain">
          {documents.map((doc) => {
            const label = DOC_TYPES.find((t) => t.value === doc.doc_type)?.label ?? doc.doc_type;
            return (
              <li key={doc.id} className="row" style={{ gap: "0.5rem", alignItems: "center" }}>
                <span className="tag">{label}</span>
                <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {doc.original_name}
                </span>
                <button
                  type="button"
                  className="btn btnDanger btnSm"
                  onClick={() => void onDeleteDocument(doc.id)}
                >
                  {en.detail.deleteDocument}
                </button>
              </li>
            );
          })}
        </ul>
      )}
      <div className="row" style={{ gap: "0.5rem", marginTop: "0.5rem", flexWrap: "wrap" }}>
        <select
          value={uploadDocType}
          onChange={(e) => setUploadDocType(e.target.value as DocType)}
          className="input"
        >
          {DOC_TYPES.map((t) => (
            <option key={t.value} value={t.value}>{t.label}</option>
          ))}
        </select>
        <label className="btn btnGhost btnSm fileImport fileImportBlock">
          <span>{en.detail.uploadDocument}</span>
          <input
            type="file"
            accept="application/pdf"
            className="visuallyHidden"
            onChange={(e) => void onUploadDocument(e.target.files?.[0])}
          />
        </label>
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
