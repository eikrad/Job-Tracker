// Documentation schema mirror for frontend typing and migrations.
export const schema = {
  jobs: [
    "id",
    "company",
    "title",
    "url",
    "raw_text",
    "status",
    "deadline",
    "tags",
    "detected_language",
    "notes",
    "pdf_path",
    "created_at",
    "updated_at",
  ],
  status_history: ["id", "job_id", "from_status", "to_status", "changed_at"],
};
