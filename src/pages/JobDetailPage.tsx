import { useEffect, useState, type ReactNode } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  ArrowLeft, Building2, Calendar, ExternalLink, FileText,
  Layers, MapPin, Monitor, Pencil, Star, Tag, Trash2,
} from "lucide-react";
import { useJobTracker } from "../context/JobTrackerContext";
import {
  deleteJobDocument, listJobDocuments, listStatusHistory,
  openDocument, openUrlInBrowser, saveJobDocument,
} from "../lib/tauriApi";
import { JobForm } from "../features/jobs/JobForm";
import { en } from "../i18n/en";
import type { DocType, JobDocument } from "../lib/types";

const PRIORITY_MAX = 10;

const DOC_TYPES: { value: DocType; label: string }[] = [
  { value: "cv", label: en.detail.docTypeCv },
  { value: "cover_letter", label: en.detail.docTypeCoverLetter },
  { value: "other", label: en.detail.docTypeOther },
];

function WorkModeIcon({ mode }: { mode?: string | null }) {
  if (mode === "Remote") return <Monitor size={14} />;
  if (mode === "On-site") return <Building2 size={14} />;
  if (mode === "Hybrid") return <Layers size={14} />;
  return null;
}

export function JobDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const {
    jobs, onDeleteJob, onUpdateJob, onExtract, statuses, syncJobList, runBackup,
  } = useJobTracker();

  const job = jobs.find((j) => j.id === Number(id));
  const [documents, setDocuments] = useState<JobDocument[]>([]);
  const [history, setHistory] = useState<Array<{ from_status: string | null; to_status: string; changed_at: string }>>([]);
  const [editing, setEditing] = useState(false);
  const [uploadDocType, setUploadDocType] = useState<DocType>("cv");

  useEffect(() => {
    if (!job) return;
    void listJobDocuments(job.id).then(setDocuments);
    void listStatusHistory(job.id).then(setHistory);
  }, [job?.id]);

  if (!job) {
    return (
      <div className="card" style={{ margin: "2rem auto", maxWidth: 480 }}>
        <p className="muted">Job not found.</p>
        <button className="btn btnGhost" onClick={() => navigate("/")}>
          <ArrowLeft size={14} style={{ marginRight: 4 }} /> {en.jobDetailPage.back}
        </button>
      </div>
    );
  }

  async function handleUpload(file?: File | null) {
    if (!file || !job) return;
    const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
    const doc = await saveJobDocument(job.id, uploadDocType, file.name, bytes);
    setDocuments((prev) => [...prev, doc]);
    await syncJobList();
    runBackup();
  }

  async function handleDeleteDocument(docId: number) {
    await deleteJobDocument(docId);
    setDocuments((prev) => prev.filter((d) => d.id !== docId));
  }

  async function handleDeleteJob() {
    if (!job) return;
    if (!window.confirm(en.alerts.deleteJobConfirm)) return;
    try {
      await onDeleteJob(job.id);
      navigate("/");
    } catch (e) {
      window.alert(en.alerts.deleteJobFailed(e instanceof Error ? e.message : String(e)));
    }
  }

  const row = (label: string, value?: string | number | null) =>
    value != null && value !== "" ? (
      <div className="detailRow">
        <span className="detailRowLabel">{label}</span>
        <span>{String(value)}</span>
      </div>
    ) : null;
  const rowNode = (label: string, node: ReactNode, show = true) =>
    show ? (
      <div className="detailRow" style={{ alignItems: "flex-start" }}>
        <span className="detailRowLabel">{label}</span>
        <span>{node}</span>
      </div>
    ) : null;

  return (
    <div className="jobDetailPage">
      <div className="jobDetailPageHeader">
        <button className="btn btnGhost btnSm" onClick={() => navigate(-1)}>
          <ArrowLeft size={14} style={{ marginRight: 4 }} />{en.jobDetailPage.back}
        </button>
        <h1>{job.company}{job.title ? ` — ${job.title}` : ""}</h1>
        <div className="row" style={{ gap: "0.5rem" }}>
          <button className="btn btnGhost btnSm" onClick={() => setEditing((v) => !v)}>
            <Pencil size={13} style={{ marginRight: 3 }} />{en.jobDetailPage.editJob}
          </button>
          <button className="btn btnDanger btnSm" onClick={() => void handleDeleteJob()}>
            <Trash2 size={13} />
          </button>
        </div>
      </div>

      {editing && (
        <div style={{ marginBottom: "1.5rem" }}>
          <JobForm
            key={job.id}
            statuses={statuses}
            editingJob={job}
            onSubmit={async () => false}
            onUpdateJob={async (jobId, payload) => {
              const ok = await onUpdateJob(jobId, payload);
              if (ok) setEditing(false);
              return ok;
            }}
            onEditClose={() => setEditing(false)}
            onExtract={onExtract}
            hideTitle
          />
        </div>
      )}

      <div className="jobDetailGrid">
        {/* Column 1 — Overview */}
        <section className="card">
          <p className="cardTitle">{en.jobDetailPage.sectionOverview}</p>
          <div className="detailRowList">
            {row(en.jobDetailPage.company, job.company)}
            {row(en.jobDetailPage.title, job.title)}
            {row(en.jobDetailPage.status, job.status)}
            {job.priority != null && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.priority}</span>
                <span>
                  {Array.from({ length: PRIORITY_MAX }, (_, i) => i + 1).map((n) => (
                    <Star key={n} size={14} fill={job.priority! >= n ? "currentColor" : "none"} style={{ display: "inline" }} />
                  ))}
                </span>
              </div>
            )}
            {job.work_mode && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.workMode}</span>
                <span><WorkModeIcon mode={job.work_mode} />{" "}{job.work_mode}</span>
              </div>
            )}
            {row(en.jobDetailPage.contractType, job.contract_type)}
            {row(en.jobDetailPage.salaryRange, job.salary_range)}
            {job.deadline && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.deadline}</span>
                <span><Calendar size={13} style={{ display: "inline", marginRight: 3 }} />{job.deadline}</span>
              </div>
            )}
            {row(en.jobDetailPage.interview, job.interview_date)}
            {row(en.jobDetailPage.start, job.start_date)}
            {job.tags && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.tags}</span>
                <span><Tag size={13} style={{ display: "inline", marginRight: 3 }} />{job.tags}</span>
              </div>
            )}
            {row(en.jobDetailPage.referenceNumber, job.reference_number)}
            {row(en.jobDetailPage.source, job.source)}
            {row(en.jobDetailPage.language, job.detected_language)}
            {row(en.jobDetailPage.createdAt, new Date(job.created_at).toLocaleString())}
            {row(en.jobDetailPage.updatedAt, new Date(job.updated_at).toLocaleString())}
          </div>
        </section>

        {/* Column 2 — Listing & Notes */}
        <section className="card">
          <p className="cardTitle">{en.jobDetailPage.sectionListing}</p>
          <div className="detailRowList">
            {rowNode(
              en.jobDetailPage.url,
              <a
                href={job.url ?? "#"}
                target="_blank"
                rel="noreferrer"
                onClick={(e) => {
                  e.preventDefault();
                  if (!job.url?.trim()) return;
                  void openUrlInBrowser(job.url.trim()).catch(console.error);
                }}
              >
                {job.url}
              </a>,
              !!job.url,
            )}
            {rowNode(
              en.jobDetailPage.notes,
              <span style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{job.notes}</span>,
              !!job.notes,
            )}
            {rowNode(
              en.jobDetailPage.rawText,
              <span style={{ whiteSpace: "pre-wrap", wordBreak: "break-word" }}>{job.raw_text}</span>,
              !!job.raw_text,
            )}
          </div>
        </section>

        {/* Column 3 — Contact & Location */}
        <section className="card">
          <p className="cardTitle">{en.jobDetailPage.sectionContact}</p>
          <div className="detailRowList">
            {job.contact_name && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.contactName}</span>
                <span>{job.contact_name}</span>
              </div>
            )}
            {job.contact_email && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.contactEmail}</span>
                <a
                  href={`mailto:${job.contact_email}`}
                  onClick={(e) => {
                    e.preventDefault();
                    void openUrlInBrowser(`mailto:${job.contact_email}`).catch(console.error);
                  }}
                >
                  {job.contact_email}
                </a>
              </div>
            )}
            {job.contact_phone && (
              <div className="detailRow">
                <span className="detailRowLabel">{en.jobDetailPage.contactPhone}</span>
                <a
                  href={`tel:${job.contact_phone}`}
                  onClick={(e) => {
                    e.preventDefault();
                    void openUrlInBrowser(`tel:${job.contact_phone}`).catch(console.error);
                  }}
                >
                  {job.contact_phone}
                </a>
              </div>
            )}
            {(job.workplace_street || job.workplace_city || job.workplace_postal_code) && (
              <div className="detailRow" style={{ alignItems: "flex-start" }}>
                <span className="detailRowLabel">{en.jobDetailPage.address}</span>
                <span>
                  <MapPin size={13} style={{ display: "inline", marginRight: 3 }} />
                  {[job.workplace_street, [job.workplace_postal_code, job.workplace_city].filter(Boolean).join(" ")].filter(Boolean).join(", ")}
                </span>
              </div>
            )}
          </div>
        </section>

        {/* Column 4 — Documents + History */}
        <section className="card">
          <p className="cardTitle">{en.jobDetailPage.sectionDocuments}</p>
          {documents.length === 0 ? (
            <p className="muted">{en.jobDetailPage.noDocuments}</p>
          ) : (
            <ul className="listPlain">
              {documents.map((doc) => {
                const label = DOC_TYPES.find((t) => t.value === doc.doc_type)?.label ?? doc.doc_type;
                return (
                  <li key={doc.id} className="row" style={{ gap: "0.5rem", alignItems: "center" }}>
                    <FileText size={14} style={{ opacity: 0.5, flexShrink: 0 }} />
                    <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{doc.original_name}</span>
                    <span className="tag">{label}</span>
                    <button
                      type="button"
                      className="btn btnGhost btnSm"
                      onClick={() => void openDocument(doc.file_path)}
                    >
                      <ExternalLink size={13} style={{ marginRight: 3 }} />{en.jobDetailPage.openDocument}
                    </button>
                    <button
                      type="button"
                      className="btn btnDanger btnSm"
                      onClick={() => void handleDeleteDocument(doc.id)}
                    >
                      <Trash2 size={13} />
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
          <div className="row" style={{ gap: "0.5rem", marginTop: "0.5rem", flexWrap: "wrap" }}>
            <select value={uploadDocType} onChange={(e) => setUploadDocType(e.target.value as DocType)} className="input">
              {DOC_TYPES.map((t) => <option key={t.value} value={t.value}>{t.label}</option>)}
            </select>
            <label className="btn btnGhost btnSm fileImport fileImportBlock">
              <span>{en.jobDetailPage.uploadDocument}</span>
              <input type="file" accept="application/pdf" className="visuallyHidden" onChange={(e) => void handleUpload(e.target.files?.[0])} />
            </label>
          </div>

          <p className="cardTitle" style={{ marginTop: "1rem" }}>{en.jobDetailPage.history}</p>
          <ul className="listPlain historyList">
            {history.map((h, idx) => (
              <li key={`${h.changed_at}-${idx}`}>
                {h.from_status ?? en.detail.newStatus} {"→"} {h.to_status} ({new Date(h.changed_at).toLocaleString()})
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
