export type JobStatus = "Interesting" | "Application Sent" | "Feedback" | "Done";

export type Job = {
  id: number;
  company: string;
  title?: string | null;
  url?: string | null;
  raw_text?: string | null;
  status: string;
  deadline?: string | null;
  /** Interview / talks / assessment day (YYYY-MM-DD). */
  interview_date?: string | null;
  /** Intended role start / contract begin (YYYY-MM-DD). */
  start_date?: string | null;
  tags?: string | null;
  detected_language?: string | null;
  notes?: string | null;
  pdf_path?: string | null;
  created_at: string;
  updated_at: string;
};

export type NewJob = {
  company: string;
  title?: string;
  url?: string;
  raw_text?: string;
  status: string;
  deadline?: string;
  interview_date?: string;
  start_date?: string;
  tags?: string;
  detected_language?: string;
  notes?: string;
};

export const DEFAULT_STATUSES: JobStatus[] = [
  "Interesting",
  "Application Sent",
  "Feedback",
  "Done",
];
