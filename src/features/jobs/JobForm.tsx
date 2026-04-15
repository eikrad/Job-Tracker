import { memo, useEffect, useState } from "react";
import { Star } from "lucide-react";
import { effectiveStatuses } from "../../lib/statusUtils";
import type { Job, NewJob } from "../../lib/types";
import { DEFAULT_STATUSES } from "../../lib/types";
import type { ExtractJobInfoResult } from "../extraction/extractJobInfo";
import { en } from "../../i18n/en";

type Props = {
  statuses: string[];
  onSubmit: (payload: NewJob) => Promise<boolean>;
  onExtract: (rawText: string) => Promise<ExtractJobInfoResult>;
  hideTitle?: boolean;
  onSubmitted?: () => void;
  /** When set, form edits this job and uses onUpdateJob instead of onSubmit. */
  editingJob?: Job | null;
  onUpdateJob?: (jobId: number, payload: NewJob) => Promise<boolean>;
  onEditClose?: () => void;
  /** Pre-fill the Job URL field (e.g. when coming from the Search page). */
  initialUrl?: string;
};

function jobToNewJob(j: Job): NewJob {
  return {
    company: j.company,
    title: j.title ?? undefined,
    url: j.url ?? undefined,
    raw_text: j.raw_text ?? undefined,
    status: j.status,
    deadline: j.deadline ?? undefined,
    interview_date: j.interview_date ?? undefined,
    start_date: j.start_date ?? undefined,
    tags: j.tags ?? undefined,
    detected_language: j.detected_language ?? undefined,
    notes: j.notes ?? undefined,
    contact_name: j.contact_name ?? undefined,
    contact_email: j.contact_email ?? undefined,
    contact_phone: j.contact_phone ?? undefined,
    workplace_street: j.workplace_street ?? undefined,
    workplace_city: j.workplace_city ?? undefined,
    workplace_postal_code: j.workplace_postal_code ?? undefined,
    work_mode: j.work_mode ?? undefined,
    salary_range: j.salary_range ?? undefined,
    contract_type: j.contract_type ?? undefined,
    priority: j.priority ?? undefined,
    reference_number: j.reference_number ?? undefined,
    source: j.source ?? undefined,
  };
}

const MERGEABLE_JOB_FIELDS: (keyof NewJob)[] = [
  "company",
  "title",
  "url",
  "raw_text",
  "deadline",
  "interview_date",
  "start_date",
  "tags",
  "detected_language",
  "notes",
  "contact_name",
  "contact_email",
  "contact_phone",
  "workplace_street",
  "workplace_city",
  "workplace_postal_code",
  "work_mode",
  "salary_range",
  "contract_type",
  "reference_number",
  "source",
];

/** Merge LLM partial fields into the form without wiping status or filling with empty/null junk. */
function mergeExtractedIntoForm(base: NewJob, partial: Partial<NewJob>): NewJob {
  const next = { ...base };
  for (const key of MERGEABLE_JOB_FIELDS) {
    const val = partial[key];
    if (val === undefined || val === null) continue;
    if (typeof val === "string" && !val.trim()) continue;
    (next as Record<string, unknown>)[key] = val;
  }
  return next;
}

export const JobForm = memo(function JobForm({
  statuses,
  onSubmit,
  onExtract,
  hideTitle = false,
  onSubmitted,
  editingJob = null,
  onUpdateJob,
  onEditClose,
  initialUrl,
}: Props) {
  const lanes = effectiveStatuses(statuses);
  const isEdit = !!editingJob;
  const [form, setForm] = useState<NewJob>(() =>
    editingJob
      ? jobToNewJob(editingJob)
      : { company: "", status: lanes[0] ?? DEFAULT_STATUSES[0], url: initialUrl },
  );
  const [error, setError] = useState("");
  const [contactOpen, setContactOpen] = useState(false);
  const [jobDetailsOpen, setJobDetailsOpen] = useState(false);
  const [extractError, setExtractError] = useState("");
  const [extractApplied, setExtractApplied] = useState(false);

  useEffect(() => {
    if (!editingJob) return;
    // Sync local form when the edited job id changes (prop → state).
    setForm(jobToNewJob(editingJob));
    setError("");
    setExtractError("");
    setExtractApplied(false);
    // Only re-sync when switching to a different job, not on every parent re-render.
  // eslint-disable-next-line react-hooks/exhaustive-deps -- editingJob identity; fields follow id
  }, [editingJob?.id]);

  const update = (patch: Partial<NewJob>) => setForm((v) => ({ ...v, ...patch }));

  async function submit() {
    if (!form.company.trim()) return setError(en.jobForm.companyRequired);
    if (!form.status) return setError(en.jobForm.statusRequired);
    setError("");
    try {
      if (isEdit && editingJob && onUpdateJob) {
        const saved = await onUpdateJob(editingJob.id, form);
        if (!saved) return;
        setExtractApplied(false);
        onEditClose?.();
        return;
      }
      const saved = await onSubmit(form);
      if (!saved) return;
      const nextLanes = effectiveStatuses(statuses);
      setForm({ company: "", status: nextLanes[0] ?? DEFAULT_STATUSES[0] });
      setExtractApplied(false);
      onSubmitted?.();
    } catch (e) {
      setError(en.jobForm.saveFailed(e instanceof Error ? e.message : String(e)));
    }
  }

  async function extract() {
    setExtractError("");
    setExtractApplied(false);
    const result = await onExtract(form.raw_text ?? "");
    if (!result.ok) {
      setExtractError(result.error);
      return;
    }
    setForm((v) => mergeExtractedIntoForm(v, result.partial));
    if (result.partial.contact_name || result.partial.contact_email || result.partial.workplace_city) {
      setContactOpen(true);
    }
    if (result.partial.work_mode || result.partial.salary_range || result.partial.contract_type) {
      setJobDetailsOpen(true);
    }
    setExtractApplied(true);
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
        <p className="muted formHint">{en.jobForm.dateFieldsLegend}</p>
        <div className="grid dateFieldsGrid">
          <label className="fieldLabelStack">
            <span className="fieldLabelText">{en.jobForm.deadlineLabel}</span>
            <input
              type="date"
              value={form.deadline ?? ""}
              onChange={(e) => update({ deadline: e.target.value })}
            />
          </label>
          <label className="fieldLabelStack">
            <span className="fieldLabelText">{en.jobForm.interviewDateLabel}</span>
            <input
              type="date"
              value={form.interview_date ?? ""}
              onChange={(e) => update({ interview_date: e.target.value })}
            />
          </label>
          <label className="fieldLabelStack">
            <span className="fieldLabelText">{en.jobForm.startDateLabel}</span>
            <input
              type="date"
              value={form.start_date ?? ""}
              onChange={(e) => update({ start_date: e.target.value })}
            />
          </label>
        </div>
      </div>
      <div className="fieldFull">
        <p className="muted formHint">{en.jobForm.extractHelp}</p>
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
      <div className="fieldFull">
        <button
          type="button"
          className="btn btnGhost sectionToggle"
          onClick={() => setContactOpen((v) => !v)}
        >
          {contactOpen ? "▾" : "▸"} {en.jobForm.contactSectionTitle}
        </button>
        {contactOpen && (
          <div className="grid" style={{ marginTop: "0.5rem" }}>
            <input placeholder={en.jobForm.contactNamePh} value={form.contact_name ?? ""} onChange={(e) => update({ contact_name: e.target.value })} />
            <input placeholder={en.jobForm.contactEmailPh} value={form.contact_email ?? ""} onChange={(e) => update({ contact_email: e.target.value })} />
            <input placeholder={en.jobForm.contactPhonePh} value={form.contact_phone ?? ""} onChange={(e) => update({ contact_phone: e.target.value })} />
            <input placeholder={en.jobForm.workplaceStreetPh} value={form.workplace_street ?? ""} onChange={(e) => update({ workplace_street: e.target.value })} />
            <input placeholder={en.jobForm.workplaceCityPh} value={form.workplace_city ?? ""} onChange={(e) => update({ workplace_city: e.target.value })} />
            <input placeholder={en.jobForm.workplacePostalCodePh} value={form.workplace_postal_code ?? ""} onChange={(e) => update({ workplace_postal_code: e.target.value })} />
          </div>
        )}
      </div>
      <div className="fieldFull">
        <button
          type="button"
          className="btn btnGhost sectionToggle"
          onClick={() => setJobDetailsOpen((v) => !v)}
        >
          {jobDetailsOpen ? "▾" : "▸"} {en.jobForm.jobDetailsSectionTitle}
        </button>
        {jobDetailsOpen && (
          <div className="grid" style={{ marginTop: "0.5rem" }}>
            <select value={form.work_mode ?? ""} onChange={(e) => update({ work_mode: e.target.value || undefined })}>
              <option value="">{en.jobForm.workModeUnknown}</option>
              <option value="Remote">{en.jobForm.workModeRemote}</option>
              <option value="Hybrid">{en.jobForm.workModeHybrid}</option>
              <option value="On-site">{en.jobForm.workModeOnSite}</option>
            </select>
            <select value={form.contract_type ?? ""} onChange={(e) => update({ contract_type: e.target.value || undefined })}>
              <option value="">{en.jobForm.contractTypeUnknown}</option>
              <option value="Permanent">{en.jobForm.contractTypePermanent}</option>
              <option value="Fixed-term">{en.jobForm.contractTypeFixedTerm}</option>
              <option value="Freelance">{en.jobForm.contractTypeFreelance}</option>
              <option value="Internship">{en.jobForm.contractTypeInternship}</option>
            </select>
            <input placeholder={en.jobForm.salaryRangePh} value={form.salary_range ?? ""} onChange={(e) => update({ salary_range: e.target.value })} />
            <input placeholder={en.jobForm.referenceNumberPh} value={form.reference_number ?? ""} onChange={(e) => update({ reference_number: e.target.value })} />
            <input placeholder={en.jobForm.sourcePh} value={form.source ?? ""} onChange={(e) => update({ source: e.target.value })} />
            <div>
              <span className="fieldLabelText">{en.jobForm.priorityLabel}</span>
              <div className="row" style={{ gap: "0.25rem", marginTop: "0.25rem" }}>
                {[1, 2, 3].map((n) => (
                  <button
                    key={n}
                    type="button"
                    className="btn btnGhost btnSm"
                    onClick={() => update({ priority: form.priority === n ? undefined : n })}
                    aria-pressed={form.priority === n}
                    style={{ padding: "0.2rem" }}
                  >
                    <Star
                      size={16}
                      fill={form.priority != null && form.priority >= n ? "currentColor" : "none"}
                    />
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
      {extractError && <p className="error extractError">{extractError}</p>}
      {extractApplied && (
        <p className="muted extractAppliedHint" role="status">
          {en.jobForm.extractApplied}
        </p>
      )}
      {error && <p className="error">{error}</p>}
      <div className="row formActions">
        {isEdit && (
          <button type="button" className="btn btnGhost" onClick={() => onEditClose?.()}>
            {en.jobForm.cancelEdit}
          </button>
        )}
        <button type="button" className="btn btnGhost" onClick={() => void extract()}>
          {en.jobForm.extractWithAi}
        </button>
        <button type="button" className="btn btnPrimary" onClick={() => void submit()}>
          {isEdit ? en.jobForm.saveChanges : en.jobForm.save}
        </button>
      </div>
    </section>
  );
});
