import type { Job } from "../../lib/types";
import { DEFAULT_STATUSES } from "../../lib/types";

type Props = {
  jobs: Job[];
  onMove: (jobId: number, status: string) => Promise<void>;
  onSelect: (job: Job) => void;
};

export function JobBoard({ jobs, onMove, onSelect }: Props) {
  return (
    <section className="board">
      {DEFAULT_STATUSES.map((status) => (
        <div key={status} className="column">
          <h3>{status}</h3>
          {jobs.filter((j) => j.status === status).map((job) => (
            <article key={job.id} className="jobCard" onClick={() => onSelect(job)}>
              <strong>{job.company}</strong>
              <span>{job.title ?? "Untitled"}</span>
              <div className="row">
                {DEFAULT_STATUSES.filter((s) => s !== job.status).map((target) => (
                  <button key={target} onClick={(e) => { e.stopPropagation(); void onMove(job.id, target); }}>
                    {target}
                  </button>
                ))}
              </div>
            </article>
          ))}
        </div>
      ))}
    </section>
  );
}
