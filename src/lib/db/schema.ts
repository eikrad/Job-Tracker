// Documentation schema mirror for frontend typing and migrations.
export const schema = {
  jobs: [
    "id", "company", "title", "url", "raw_text", "status",
    "deadline", "interview_date", "start_date",
    "tags", "detected_language", "notes",
    "contact_name", "contact_email", "contact_phone",
    "workplace_street", "workplace_city", "workplace_postal_code",
    "work_mode", "salary_range", "contract_type",
    "priority", "reference_number", "source",
    "pdf_path", "created_at", "updated_at",
  ],
  status_history: ["id", "job_id", "from_status", "to_status", "changed_at"],
  job_documents: ["id", "job_id", "doc_type", "original_name", "file_path", "created_at"],
};
