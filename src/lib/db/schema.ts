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
    "listing_status", "listing_checked_at",
    "mail_score", "mail_score_reason", "mail_scored_at",
    "pdf_path", "created_at", "updated_at",
  ],
  status_history: ["id", "job_id", "from_status", "to_status", "changed_at"],
  job_documents: ["id", "job_id", "doc_type", "original_name", "file_path", "created_at"],
  mail_scan_runs: [
    "run_id", "trigger", "status", "started_at", "finished_at", "stats_json",
    "error_code", "error_summary", "sidecar_version", "model_id",
    "profile_short_hash", "profile_full_hash",
  ],
  mail_fingerprints: [
    "fingerprint_id", "strong_key", "weak_key", "first_seen_at", "last_seen_at", "seen_count",
  ],
  mail_fingerprint_aliases: ["alias_id", "fingerprint_id", "created_at"],
  mail_match_inbox: [
    "id", "fingerprint_id", "kind", "status", "job_id", "score", "score_reason",
    "score_state", "suspicious", "near_duplicate_of", "draft_json", "enrichment_state",
    "enrichment_error", "source_board", "message_id", "message_date", "listing_url",
    "base_job_updated_at", "first_run_id", "last_run_id", "created_at", "updated_at",
  ],
  mail_match_dismissals: [
    "fingerprint_id", "scope", "reason", "dismissed_at", "dismissed_run",
  ],
  mail_scored_sightings: [
    "id", "fingerprint_id", "pass", "score", "reason", "profile_hash",
    "prompt_version", "model_id", "listing_content_hash", "outcome", "scored_at", "run_id",
  ],
  mail_source_cursors: [
    "source_id", "path", "kind", "size", "mtime_ns", "offset",
    "sentinel_hash", "last_message_id", "updated_at",
  ],
  job_field_provenance: ["job_id", "field", "source", "run_id", "set_at"],
};
