import { memo } from "react";
import { Star, Calendar, Monitor, Tag, ArrowRight, Trash2 } from "lucide-react";
import { en } from "../../i18n/en";
import type { Job } from "../../lib/types";

const PRIORITY_MAX = 10;

type Props = {
  selected?: Job;
  onDeleteJob: (jobId: number) => Promise<void>;
  onViewDetails: (jobId: number) => void;
};

export const JobDetailTimeline = memo(function JobDetailTimeline({
  selected,
  onDeleteJob,
  onViewDetails,
}: Props) {
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
      <section className="card dashboardPanel">
        <h2>{en.detail.title}</h2>
        <p className="muted">{en.detail.selectJob}</p>
      </section>
    );
  }

  return (
    <section className="card dashboardPanel">
      <h2>{en.detail.title}</h2>
      <div className="dashboardPanelScroll">
        <p className="detailMeta">
          <strong>{selected.company}</strong>
          {selected.title ? ` — ${selected.title}` : ""}
        </p>
        <p className="detailMeta">
          {en.detail.status} <span>{selected.status}</span>
          {selected.priority != null && (
            <span style={{ marginLeft: "0.5rem" }}>
              {Array.from({ length: PRIORITY_MAX }, (_, i) => i + 1).map((n) => (
                <Star key={n} size={13} fill={selected.priority! >= n ? "currentColor" : "none"} style={{ display: "inline" }} />
              ))}
            </span>
          )}
        </p>
        {(selected.deadline || selected.interview_date || selected.start_date) && (
          <p className="detailMeta detailDates">
            {selected.deadline && <><Calendar size={13} style={{ display: "inline", marginRight: 3 }} />{en.detail.deadlineShort}: <span>{selected.deadline}</span></>}
            {selected.interview_date && <>{selected.deadline ? " · " : null}{en.detail.interviewShort}: <span>{selected.interview_date}</span></>}
            {selected.start_date && <>{(selected.deadline || selected.interview_date) ? " · " : null}{en.detail.startShort}: <span>{selected.start_date}</span></>}
          </p>
        )}
        {(selected.work_mode || selected.contract_type) && (
          <p className="detailMeta">
            <Monitor size={13} style={{ display: "inline", marginRight: 3 }} />
            {[selected.work_mode, selected.contract_type].filter(Boolean).join(" · ")}
          </p>
        )}
        {selected.tags && (
          <p className="detailMeta">
            <Tag size={13} style={{ display: "inline", marginRight: 3 }} />
            {selected.tags}
          </p>
        )}
        {selected.notes && <p className="detailMeta">{en.detail.notes}: {selected.notes}</p>}
        <div className="detailActions row" style={{ marginTop: "0.75rem" }}>
          <button type="button" className="btn btnPrimary btnSm" onClick={() => onViewDetails(selected.id)}>
            {en.detail.viewFullDetails} <ArrowRight size={13} style={{ display: "inline", marginLeft: 3 }} />
          </button>
          <button type="button" className="btn btnDanger btnSm" onClick={() => void onDelete()}>
            <Trash2 size={13} />
          </button>
        </div>
      </div>
    </section>
  );
});
