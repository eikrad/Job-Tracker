import { useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import "./App.css";
import { AppHeader } from "./components/AppHeader";
import { SettingsModal } from "./components/SettingsModal";
import { JobTrackerProvider } from "./context/JobTrackerContext";
import { useJobTrackerState } from "./hooks/useJobTrackerState";
import { DashboardPage } from "./pages/DashboardPage";
import { AddJobPage } from "./pages/AddJobPage";
import { JobDetailPage } from "./pages/JobDetailPage";
import { JobSearchPage } from "./pages/JobSearchPage";

export default function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const state = useJobTrackerState({
    openSettings: () => setSettingsOpen(true),
  });

  return (
    <BrowserRouter>
      <JobTrackerProvider value={state}>
        <AppHeader onOpenSettings={() => setSettingsOpen(true)} />
        <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
        <main className="app">
          <Routes>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/jobs/new" element={<AddJobPage />} />
            <Route path="/job/:id" element={<JobDetailPage />} />
            <Route path="/job-search" element={<JobSearchPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </main>
      </JobTrackerProvider>
    </BrowserRouter>
  );
}
