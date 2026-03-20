import { memo } from "react";
import type { Job } from "../../lib/types";
import { WorkspaceEmpty } from "../../components/WorkspaceEmpty";
import { en } from "../../i18n/en";

type Props = { jobs: Job[]; onSelect: (job: Job) => void };

export const JobTable = memo(function JobTable({ jobs, onSelect }: Props) {
  return (
    <section className="card">
      <h2>{en.jobTable.title}</h2>
      {jobs.length === 0 ? (
        <WorkspaceEmpty
          title={en.empty.tableTitle}
          body={en.empty.tableBody}
          cta={en.empty.tableCta}
        />
      ) : (
        <div className="tableWrap">
          <table>
            <thead>
              <tr>
                <th>{en.jobTable.company}</th>
                <th>{en.jobTable.titleCol}</th>
                <th>{en.jobTable.status}</th>
                <th>{en.jobTable.deadline}</th>
                <th>{en.jobTable.interview}</th>
                <th>{en.jobTable.start}</th>
                <th>{en.jobTable.language}</th>
              </tr>
            </thead>
            <tbody>
              {jobs.map((job) => (
                <tr key={job.id} onClick={() => onSelect(job)}>
                  <td>{job.company}</td>
                  <td>{job.title ?? en.common.dash}</td>
                  <td>{job.status}</td>
                  <td>{job.deadline ?? en.common.dash}</td>
                  <td>{job.interview_date ?? en.common.dash}</td>
                  <td>{job.start_date ?? en.common.dash}</td>
                  <td>{job.detected_language ?? en.common.dash}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
});
