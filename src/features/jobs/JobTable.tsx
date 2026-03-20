import type { Job } from "../../lib/types";

type Props = { jobs: Job[]; onSelect: (job: Job) => void };

export function JobTable({ jobs, onSelect }: Props) {
  return (
    <section className="card">
      <h2>Table View</h2>
      <table>
        <thead>
          <tr>
            <th>Company</th>
            <th>Title</th>
            <th>Status</th>
            <th>Deadline</th>
            <th>Language</th>
          </tr>
        </thead>
        <tbody>
          {jobs.map((job) => (
            <tr key={job.id} onClick={() => onSelect(job)}>
              <td>{job.company}</td>
              <td>{job.title ?? "-"}</td>
              <td>{job.status}</td>
              <td>{job.deadline ?? "-"}</td>
              <td>{job.detected_language ?? "-"}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
