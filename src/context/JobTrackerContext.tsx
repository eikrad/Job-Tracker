import { createContext, useContext, type ReactNode } from "react";
import type { JobTrackerState } from "../hooks/useJobTrackerState";

const JobTrackerContext = createContext<JobTrackerState | null>(null);

export function JobTrackerProvider({ value, children }: { value: JobTrackerState; children: ReactNode }) {
  return <JobTrackerContext.Provider value={value}>{children}</JobTrackerContext.Provider>;
}

/** Colocated with provider; split files would duplicate context wiring. */
// eslint-disable-next-line react-refresh/only-export-components -- useJobTracker is not a component
export function useJobTracker(): JobTrackerState {
  const ctx = useContext(JobTrackerContext);
  if (!ctx) throw new Error("useJobTracker must be used within JobTrackerProvider");
  return ctx;
}
