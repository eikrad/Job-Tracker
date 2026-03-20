import type { ReactNode } from "react";
import { memo, useMemo } from "react";
import {
  DndContext,
  type DragEndEvent,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import { groupJobsByStatus } from "../../lib/jobs/groupJobsByStatus";
import { effectiveStatuses } from "../../lib/statusUtils";
import type { Job } from "../../lib/types";
import { WorkspaceEmpty } from "../../components/WorkspaceEmpty";
import { en } from "../../i18n/en";

type Props = {
  statuses: string[];
  jobs: Job[];
  onMove: (jobId: number, status: string) => Promise<void>;
  onSelect: (job: Job) => void;
};

function laneId(status: string) {
  return `lane::${status}`;
}

function jobDragId(jobId: number) {
  return `job::${jobId}`;
}

function Lane({
  status,
  children,
}: {
  status: string;
  children: ReactNode;
}) {
  const { setNodeRef, isOver } = useDroppable({ id: laneId(status) });
  return (
    <div ref={setNodeRef} className={`column ${isOver ? "columnOver" : ""}`}>
      <h3>{status}</h3>
      {children}
    </div>
  );
}

function JobCard({
  job,
  lanes,
  onMove,
  onSelect,
}: {
  job: Job;
  lanes: string[];
  onMove: (jobId: number, status: string) => Promise<void>;
  onSelect: (job: Job) => void;
}) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: jobDragId(job.id),
    data: { jobId: job.id },
  });
  const style = transform
    ? { transform: `translate3d(${transform.x}px,${transform.y}px,0)` }
    : undefined;

  return (
    <article
      ref={setNodeRef}
      style={style}
      className={`jobCard ${isDragging ? "jobCardDragging" : ""}`}
      onClick={() => onSelect(job)}
    >
      <div className="dragHandle" {...listeners} {...attributes} title={en.jobBoard.dragToMoveStatus}>
        ⋮⋮
      </div>
      <strong>{job.company}</strong>
      <span>{job.title ?? en.common.untitled}</span>
      {(job.deadline || job.interview_date || job.start_date) && (
        <div className="muted jobCardDates">
          {job.deadline && <span>{en.deadlines.dateLineApply}: {job.deadline}</span>}
          {job.interview_date && (
            <span>
              {job.deadline ? " · " : ""}
              {en.deadlines.dateLineInterview}: {job.interview_date}
            </span>
          )}
          {job.start_date && (
            <span>
              {job.deadline || job.interview_date ? " · " : ""}
              {en.deadlines.dateLineStart}: {job.start_date}
            </span>
          )}
        </div>
      )}
      <div className="row">
        {lanes
          .filter((s) => s !== job.status)
          .map((target) => (
            <button
              key={target}
              type="button"
              className="btn btnSm btnGhost"
              onClick={(e) => {
                e.stopPropagation();
                void onMove(job.id, target);
              }}
            >
              {target}
            </button>
          ))}
      </div>
    </article>
  );
}

export const JobBoard = memo(function JobBoard({ statuses, jobs, onMove, onSelect }: Props) {
  const lanes = effectiveStatuses(statuses);
  const jobsByStatus = useMemo(() => groupJobsByStatus(jobs), [jobs]);
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 6 },
    }),
  );

  function onDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over) return;
    const overId = String(over.id);
    const activeId = String(active.id);
    if (!overId.startsWith("lane::")) return;
    const newStatus = overId.replace(/^lane::/, "");
    const jobId = Number(activeId.replace(/^job::/, ""));
    if (!Number.isFinite(jobId)) return;
    void onMove(jobId, newStatus);
  }

  return (
    <section className="card">
      <h2>{en.jobBoard.sectionTitle}</h2>
      {jobs.length === 0 ? (
        <WorkspaceEmpty
          title={en.empty.boardTitle}
          body={en.empty.boardBody}
          cta={en.empty.boardCta}
        />
      ) : (
        <div className="boardWrap">
          <DndContext sensors={sensors} onDragEnd={onDragEnd}>
            <section className="board">
              {lanes.map((status) => (
                <Lane key={status} status={status}>
                  {(jobsByStatus.get(status) ?? []).map((job) => (
                    <JobCard
                      key={job.id}
                      job={job}
                      lanes={lanes}
                      onMove={onMove}
                      onSelect={onSelect}
                    />
                  ))}
                </Lane>
              ))}
            </section>
          </DndContext>
        </div>
      )}
    </section>
  );
});
