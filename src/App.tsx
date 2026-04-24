import { useEffect, useState } from "react";
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import "./App.css";
import { AppHeader } from "./components/AppHeader";
import { QuickCaptureDrawer } from "./components/QuickCaptureDrawer";
import { SettingsModal } from "./components/SettingsModal";
import { JobTrackerProvider } from "./context/JobTrackerContext";
import { useJobTrackerState } from "./hooks/useJobTrackerState";
import { DashboardPage } from "./pages/DashboardPage";
import { AddJobPage } from "./pages/AddJobPage";
import { JobDetailPage } from "./pages/JobDetailPage";
import { JobSearchPage } from "./pages/JobSearchPage";
import { enqueueBrowserCaptureUrl } from "./features/capture/captureInbox";

export default function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [quickCaptureOpen, setQuickCaptureOpen] = useState(false);
  const state = useJobTrackerState({
    openSettings: () => setSettingsOpen(true),
  });

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const captureUrl = params.get("capture_url");
    if (!captureUrl) return;
    enqueueBrowserCaptureUrl(captureUrl);
    params.delete("capture_url");
    const nextQuery = params.toString();
    const nextUrl = `${window.location.pathname}${nextQuery ? `?${nextQuery}` : ""}${window.location.hash}`;
    window.history.replaceState({}, "", nextUrl);
  }, []);

  return (
    <BrowserRouter>
      <JobTrackerProvider value={state}>
        <AppHeader
          onOpenSettings={() => setSettingsOpen(true)}
          onOpenQuickCapture={() => setQuickCaptureOpen(true)}
        />
        <QuickCaptureDrawer open={quickCaptureOpen} onClose={() => setQuickCaptureOpen(false)} />
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
