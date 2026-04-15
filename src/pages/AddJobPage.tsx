import { Link, useLocation, useNavigate } from "react-router-dom";
import { JobForm } from "../features/jobs/JobForm";
import { useJobTracker } from "../context/JobTrackerContext";
import { en } from "../i18n/en";

interface AddJobRouteState {
  /** Pre-fill the URL field when arriving from the Search page. */
  prefillUrl?: string;
}

export function AddJobPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const routeState = location.state as AddJobRouteState | null;
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
        initialUrl={routeState?.prefillUrl}
      />
    </div>
  );
}
