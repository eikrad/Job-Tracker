import { useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import "./App.css";
import { AppHeader } from "./components/AppHeader";
import { SettingsModal } from "./components/SettingsModal";
import { JobTrackerProvider } from "./context/JobTrackerContext";
import { useJobTrackerState } from "./hooks/useJobTrackerState";
import { DashboardPage } from "./pages/DashboardPage";
import { AddJobPage } from "./pages/AddJobPage";

export default function App() {
  const state = useJobTrackerState();
  const [settingsOpen, setSettingsOpen] = useState(false);

  return (
    <BrowserRouter>
      <JobTrackerProvider value={state}>
        <AppHeader onOpenSettings={() => setSettingsOpen(true)} />
        <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
        <main className="app">
          <Routes>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/jobs/new" element={<AddJobPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </main>
      </JobTrackerProvider>
    </BrowserRouter>
  );
}
