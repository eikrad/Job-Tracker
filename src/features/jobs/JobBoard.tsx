import type { ReactNode } from "react";
import {
  DndContext,
  type DragEndEvent,
  PointerSensor,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { Job } from "../../lib/types";
import { DEFAULT_STATUSES } from "../../lib/types";

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
      <div className="dragHandle" {...listeners} {...attributes} title="Drag to move status">
        ⋮⋮
      </div>
      <strong>{job.company}</strong>
      <span>{job.title ?? "Untitled"}</span>
      <div className="row">
        {lanes
          .filter((s) => s !== job.status)
          .map((target) => (
            <button
              key={target}
              type="button"
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

export function JobBoard({ statuses, jobs, onMove, onSelect }: Props) {
  const lanes = statuses.length ? statuses : DEFAULT_STATUSES;
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
    <DndContext sensors={sensors} onDragEnd={onDragEnd}>
      <section className="board">
        {lanes.map((status) => (
          <Lane key={status} status={status}>
            {jobs
              .filter((j) => j.status === status)
              .map((job) => (
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
  );
}
