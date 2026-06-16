import { Link, NavLink, useLocation } from "react-router-dom";
import { Monitor, Moon, Sun } from "lucide-react";
import { useJobTracker } from "../context/JobTrackerContext";
import { useTheme } from "../hooks/useTheme";
import type { ThemePreference } from "../lib/theme";
import { en } from "../i18n/en";

type Props = {
  onOpenSettings: () => void;
  onOpenQuickCapture: () => void;
};

const THEME_CYCLE: ThemePreference[] = ["system", "light", "dark"];

const THEME_META: Record<
  ThemePreference,
  { icon: typeof Monitor; label: string }
> = {
  system: { icon: Monitor, label: en.app.themeSystem },
  light: { icon: Sun, label: en.app.themeLight },
  dark: { icon: Moon, label: en.app.themeDark },
};

export function AppHeader({ onOpenSettings, onOpenQuickCapture }: Props) {
  const { view, setView } = useJobTracker();
  const { preference, setPreference } = useTheme();
  const location = useLocation();
  const isDashboard = location.pathname === "/";

  const themeMeta = THEME_META[preference];
  const ThemeIcon = themeMeta.icon;
  const cycleTheme = () => {
    const next = THEME_CYCLE[(THEME_CYCLE.indexOf(preference) + 1) % THEME_CYCLE.length];
    setPreference(next);
  };

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
              onClick={onOpenQuickCapture}
              aria-label={en.capture.openDrawer}
            >
              {en.capture.openDrawer}
            </button>
            <button
              type="button"
              className="btn btnGhost btnIcon"
              onClick={cycleTheme}
              aria-label={en.app.themeToggleAria(themeMeta.label)}
              title={en.app.themeToggleAria(themeMeta.label)}
            >
              <ThemeIcon size={16} />
            </button>
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
