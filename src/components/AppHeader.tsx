import { Link, NavLink, useLocation } from "react-router-dom";
import { useJobTracker } from "../context/JobTrackerContext";
import { en } from "../i18n/en";

type Props = {
  onOpenSettings: () => void;
};

export function AppHeader({ onOpenSettings }: Props) {
  const { view, setView } = useJobTracker();
  const location = useLocation();
  const isDashboard = location.pathname === "/";

  return (
    <header className="appHeader">
      <div className="appHeaderInner">
        <Link to="/" className="appBrandLink" aria-label={en.app.navHomeAria}>
          <div className="appBrand">
            <h1>
              {en.app.title}
              <span className="appBadge">{en.app.stageAlpha}</span>
            </h1>
            <p className="appTagline">{en.app.tagline}</p>
          </div>
        </Link>
        <div className="appHeaderActions">
          {isDashboard && (
            <nav className="appNav" aria-label={en.app.navAriaMainViews}>
              <div className="tabList" role="tablist">
                <button
                  type="button"
                  role="tab"
                  className="btnTab"
                  aria-selected={view === "kanban"}
                  onClick={() => setView("kanban")}
                >
                  {en.nav.kanban}
                </button>
                <button
                  type="button"
                  role="tab"
                  className="btnTab"
                  aria-selected={view === "table"}
                  onClick={() => setView("table")}
                >
                  {en.nav.table}
                </button>
                <button
                  type="button"
                  role="tab"
                  className="btnTab"
                  aria-selected={view === "calendar"}
                  onClick={() => setView("calendar")}
                >
                  {en.nav.calendar}
                </button>
              </div>
            </nav>
          )}
            <div className="appToolbar appToolbarEnd" role="toolbar" aria-label={en.app.navAriaToolbar}>
            <NavLink
              to="/job-search"
              className={({ isActive }) => `btn ${isActive ? "btnPrimary" : "btnGhost"}`}
            >
              {en.jobSearch.navLink}
            </NavLink>
            <NavLink
              to="/jobs/new"
              className={({ isActive }) => `btn ${isActive ? "btnPrimary" : "btnGhost"}`}
              aria-label={en.app.navAddJobAria}
            >
              {en.nav.addJob}
            </NavLink>
            <button
              type="button"
              className="btn btnGhost"
              onClick={onOpenSettings}
              aria-label={en.app.navSettingsAria}
            >
              {en.app.settingsOpen}
            </button>
          </div>
        </div>
      </div>
    </header>
  );
}
