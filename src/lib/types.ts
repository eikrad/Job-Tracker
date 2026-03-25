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
  contact_name?: string | null;
  contact_email?: string | null;
  contact_phone?: string | null;
  workplace_street?: string | null;
  workplace_city?: string | null;
  workplace_postal_code?: string | null;
  work_mode?: string | null;
  salary_range?: string | null;
  contract_type?: string | null;
  priority?: number | null;
  reference_number?: string | null;
  source?: string | null;
  pdf_path?: string | null;
  created_at: string;
  updated_at: string;
};

export type DocType = "cv" | "cover_letter" | "other";

export type JobDocument = {
  id: number;
  job_id: number;
  doc_type: DocType;
  original_name: string;
  file_path: string;
  created_at: string;
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
  contact_name?: string;
  contact_email?: string;
  contact_phone?: string;
  workplace_street?: string;
  workplace_city?: string;
  workplace_postal_code?: string;
  work_mode?: string;
  salary_range?: string;
  contract_type?: string;
  priority?: number;
  reference_number?: string;
  source?: string;
};

export const DEFAULT_STATUSES: JobStatus[] = [
  "Interesting",
  "Application Sent",
  "Feedback",
  "Done",
];
