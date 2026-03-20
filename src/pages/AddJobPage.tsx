import { Link, useNavigate } from "react-router-dom";
import { JobForm } from "../features/jobs/JobForm";
import { useJobTracker } from "../context/JobTrackerContext";
import { en } from "../i18n/en";

export function AddJobPage() {
  const navigate = useNavigate();
  const { statuses, onSubmit, onExtract } = useJobTracker();

  return (
    <div className="addJobPage">
      <div className="addJobPageTop">
        <Link to="/" className="addJobBack btn btnGhost">
          ← {en.addJobPage.back}
        </Link>
        <h1 className="addJobPageTitle">{en.addJobPage.title}</h1>
      </div>
      <JobForm
        statuses={statuses}
        onSubmit={onSubmit}
        onExtract={onExtract}
        hideTitle
        onSubmitted={() => navigate("/")}
      />
    </div>
  );
}
