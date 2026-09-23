//! Accepting a match into the Jobs table (spec §5.2).
//!
//! Two properties this module exists to hold, each of which is a silent data-loss bug
//! if it breaks:
//!
//! 1. **Idempotency.** Accept moves the row `pending → accepted` in the same
//!    transaction as the job write, guarded by `WHERE status = 'pending'`. A double
//!    click cannot create two Jobs.
//! 2. **Provenance.** Every field written from a scan gets a `job_field_provenance`
//!    row, so "where did this deadline come from?" has an answer.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::mail_scan::status::InboxStatus;

/// Job fields a scan-created Job may take from its Draft; each one written gets a
/// provenance row.
///
/// `status` and `priority` are absent and must stay absent: they are the user's
/// workflow state, and nothing derived from an email gets to move a job through it
/// (spec §0.1, §5.2).
pub const DRAFT_FIELDS: &[&str] = &[
    "title",
    "url",
    "board_url",
    "deadline",
    "interview_date",
    "start_date",
    "tags",
    "detected_language",
    "notes",
    "contact_name",
    "contact_email",
    "contact_phone",
    "workplace_street",
    "workplace_city",
    "workplace_postal_code",
    "work_mode",
    "salary_range",
    "contract_type",
    "reference_number",
    "source",
];

fn record_provenance(
    tx: &Connection,
    job_id: i64,
    field: &str,
    run_id: Option<&str>,
    now: &str,
) -> Result<(), String> {
    tx.execute(
        "INSERT OR REPLACE INTO job_field_provenance (job_id, field, source, run_id, set_at)
         VALUES (?1, ?2, 'mail_scan', ?3, ?4)",
        params![job_id, field, run_id, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Mark the inbox row accepted, guarded by its current status.
///
/// Returns false when the row was not `pending` — i.e. somebody already accepted it.
/// Called inside the same transaction as the job write, which is what makes a
/// double-click safe.
fn claim_pending(tx: &Connection, inbox_id: i64, job_id: i64, now: &str) -> Result<bool, String> {
    let changed = tx
        .execute(
            "UPDATE mail_match_inbox
             SET status = ?4, job_id = ?1, updated_at = ?2
             WHERE id = ?3 AND status = ?5",
            params![job_id, now, inbox_id, InboxStatus::Accepted, InboxStatus::Pending],
        )
        .map_err(|e| e.to_string())?;
    Ok(changed == 1)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptOutcome {
    pub job_id: i64,
    /// Fields the Job took from the Draft; empty when a double-click lost the race.
    pub fields_written: Vec<String>,
    /// True when this call did the work; false when it lost the race to a double-click.
    pub created: bool,
}

/// Copy the match's advisory score onto the Job. Never touches `priority` (spec §0.1).
fn apply_mail_score(
    tx: &Connection,
    job_id: i64,
    inbox_id: i64,
    now: &str,
) -> Result<(), String> {
    let score: (Option<i64>, Option<String>) = tx
        .query_row(
            "SELECT score, score_reason FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE jobs SET mail_score = ?1, mail_score_reason = ?2, mail_scored_at = ?3
         WHERE id = ?4",
        params![score.0, score.1, now, job_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Stand-in when neither the digest nor the listing page named a company. `jobs.company`
/// is required, and refusing the accept would strand the match; the user fixes the
/// name on the Job, which is easier than retyping the whole Draft into a form.
pub const UNKNOWN_COMPANY: &str = "Unknown company";

/// Map a stored Draft onto the Job it becomes.
///
/// Blank values stay `None` so provenance is only claimed for fields that carry
/// something. `status` and `priority` are never read from the Draft: a scan-created Job
/// always starts in `Interesting` (spec §5.2), and priority is the user's (spec §0.1).
fn job_from_draft(draft: &HashMap<String, Value>, board: Option<&str>) -> crate::db::NewJob {
    let text = |field: &str| {
        draft
            .get(field)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    crate::db::NewJob {
        company: text("company").unwrap_or_else(|| UNKNOWN_COMPANY.to_string()),
        title: text("title"),
        url: text("url"),
        raw_text: text("raw_text"),
        status: "Interesting".to_string(),
        deadline: text("deadline"),
        interview_date: text("interview_date"),
        start_date: text("start_date"),
        tags: text("tags"),
        detected_language: text("detected_language"),
        notes: text("notes"),
        contact_name: text("contact_name"),
        contact_email: text("contact_email"),
        contact_phone: text("contact_phone"),
        workplace_street: text("workplace_street"),
        workplace_city: text("workplace_city"),
        workplace_postal_code: text("workplace_postal_code"),
        work_mode: text("work_mode"),
        salary_range: text("salary_range"),
        contract_type: text("contract_type"),
        priority: None,
        reference_number: text("reference_number"),
        source: text("source").or_else(|| board.map(str::to_string)),
        board_url: text("board_url"),
    }
}

/// Create a Job from a match in one step, straight from its stored Draft.
///
/// The inbox row is claimed in the same transaction as the insert, so two rapid
/// accepts produce one Job and one `accepted` row. [`undo_accept`] is the way back.
pub fn accept_new(conn: &mut Connection, inbox_id: i64) -> Result<AcceptOutcome, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Claim first: if this loses the race, no Job is created at all.
    type Row = (InboxStatus, Option<i64>, String, Option<String>, Option<String>);
    let row: Option<Row> = tx
        .query_row(
            "SELECT status, job_id, draft_json, source_board, last_run_id
             FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((status, existing_job, draft_json, board, run_id)) = row else {
        return Err("That inbox row no longer exists.".into());
    };
    if status != InboxStatus::Pending {
        tx.rollback().map_err(|e| e.to_string())?;
        return Ok(AcceptOutcome {
            job_id: existing_job.unwrap_or_default(),
            fields_written: Vec::new(),
            created: false,
        });
    }

    let draft: HashMap<String, Value> = serde_json::from_str(&draft_json)
        .map_err(|e| format!("Could not read the stored draft: {e}"))?;
    let to_insert = job_from_draft(&draft, board.as_deref());

    let job_id = crate::db::insert_job_returning_id(&tx, &to_insert, &now)?;

    let mut written = Vec::new();
    for field in DRAFT_FIELDS {
        if payload_field_is_set(&to_insert, field) {
            record_provenance(&tx, job_id, field, run_id.as_deref(), &now)?;
            written.push((*field).to_string());
        }
    }

    if !claim_pending(&tx, inbox_id, job_id, &now)? {
        tx.rollback().map_err(|e| e.to_string())?;
        return Ok(AcceptOutcome {
            job_id: existing_job.unwrap_or_default(),
            fields_written: Vec::new(),
            created: false,
        });
    }
    apply_mail_score(&tx, job_id, inbox_id, &now)?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(AcceptOutcome {
        job_id,
        fields_written: written,
        created: true,
    })
}

/// Take back a one-click accept: delete the Job it created (and its provenance) and
/// return the match to `pending`.
///
/// Allowed for as long as the Job exists — edits made in the meantime go with it, which
/// is what "undo" means right after an accept. The one refusal is a Job with documents
/// attached: those are files on disk, and deleting them belongs to the job page, which
/// knows how to clean them up.
pub fn undo_accept(conn: &mut Connection, inbox_id: i64) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let row: Option<(InboxStatus, Option<i64>)> = tx
        .query_row(
            "SELECT status, job_id FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    // `mail_match_inbox.job_id` cascades, so a Job deleted since the accept has
    // already taken the row with it.
    let Some((status, job_id)) = row else {
        return Err("That match is gone; its job was probably deleted already.".into());
    };
    let (true, Some(job_id)) = (status == InboxStatus::Accepted, job_id) else {
        return Err(format!("That match is {status}, not accepted."));
    };

    let documents: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM job_documents WHERE job_id = ?1",
            params![job_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if documents > 0 {
        return Err("That job has documents attached; delete it from its job page.".into());
    }

    // Unlink first: deleting the Job while the row still points at it would cascade
    // the row away too.
    tx.execute(
        "UPDATE mail_match_inbox SET status = ?3, job_id = NULL, updated_at = ?1
         WHERE id = ?2",
        params![&now, inbox_id, InboxStatus::Pending],
    )
    .map_err(|e| e.to_string())?;
    for sql in [
        "DELETE FROM job_field_provenance WHERE job_id = ?1",
        "DELETE FROM status_history WHERE job_id = ?1",
        "DELETE FROM jobs WHERE id = ?1",
    ] {
        tx.execute(sql, params![job_id]).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())
}

fn payload_field_is_set(payload: &crate::db::NewJob, field: &str) -> bool {
    let get = |v: &Option<String>| v.as_deref().map(str::trim).is_some_and(|s| !s.is_empty());
    match field {
        "title" => get(&payload.title),
        "url" => get(&payload.url),
        "board_url" => get(&payload.board_url),
        "deadline" => get(&payload.deadline),
        "interview_date" => get(&payload.interview_date),
        "start_date" => get(&payload.start_date),
        "tags" => get(&payload.tags),
        "detected_language" => get(&payload.detected_language),
        "notes" => get(&payload.notes),
        "contact_name" => get(&payload.contact_name),
        "contact_email" => get(&payload.contact_email),
        "contact_phone" => get(&payload.contact_phone),
        "workplace_street" => get(&payload.workplace_street),
        "workplace_city" => get(&payload.workplace_city),
        "workplace_postal_code" => get(&payload.workplace_postal_code),
        "work_mode" => get(&payload.work_mode),
        "salary_range" => get(&payload.salary_range),
        "contract_type" => get(&payload.contract_type),
        "reference_number" => get(&payload.reference_number),
        "source" => get(&payload.source),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use serde_json::json;

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO mail_scan_runs (run_id, status, started_at) VALUES ('r1','completed','t')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mail_fingerprints (fingerprint_id, strong_key, weak_key, first_seen_at, last_seen_at)
             VALUES ('fp1','s1','w1','t','t')",
            [],
        )
        .unwrap();
        conn
    }

    fn seed_inbox(conn: &Connection, draft: serde_json::Value) -> i64 {
        conn.execute(
            "INSERT INTO mail_match_inbox (
                fingerprint_id, kind, status, score, score_reason, score_state,
                draft_json, enrichment_state, first_run_id, last_run_id, created_at, updated_at
             ) VALUES ('fp1', 'new', 'pending', 8, 'good match', 'ok',
                       ?1, 'complete', 'r1','r1','t','t')",
            params![draft.to_string()],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // ----- One-click accept: the Draft becomes the Job ----------------------

    fn full_draft() -> serde_json::Value {
        json!({
            "title": "Rust Engineer",
            "company": "Acme",
            "url": "https://example.com/job",
            "raw_text": "We are hiring a Rust engineer to build the ingestion pipeline.",
            "deadline": "2026-10-01",
            "contact_name": "Ada",
            "workplace_city": "København",
            "work_mode": "Hybrid",
            "source": "indeed",
        })
    }

    #[test]
    fn accepting_a_pending_match_creates_an_interesting_job_from_its_draft() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, full_draft());

        let outcome = accept_new(&mut conn, inbox).unwrap();

        assert!(outcome.created);
        let job: (String, String, String, String, String, String, String, String, Option<i64>) =
            conn.query_row(
                "SELECT company, title, url, raw_text, deadline, work_mode, source, status,
                        mail_score
                 FROM jobs WHERE id = ?1",
                params![outcome.job_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            job,
            (
                "Acme".to_string(),
                "Rust Engineer".to_string(),
                "https://example.com/job".to_string(),
                "We are hiring a Rust engineer to build the ingestion pipeline.".to_string(),
                "2026-10-01".to_string(),
                "Hybrid".to_string(),
                "indeed".to_string(),
                "Interesting".to_string(),
                Some(8),
            )
        );
        let status: String = conn
            .query_row(
                "SELECT status FROM mail_match_inbox WHERE id = ?1",
                params![inbox],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "accepted");
    }

    #[test]
    fn undoing_an_accept_removes_the_job_and_returns_the_match_to_pending() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, full_draft());
        let outcome = accept_new(&mut conn, inbox).unwrap();

        undo_accept(&mut conn, inbox).unwrap();

        let count = |sql: &str| -> i64 {
            conn.query_row(sql, params![outcome.job_id], |r| r.get(0)).unwrap()
        };
        assert_eq!(count("SELECT COUNT(*) FROM jobs WHERE id = ?1"), 0);
        assert_eq!(
            count("SELECT COUNT(*) FROM job_field_provenance WHERE job_id = ?1"),
            0,
            "a removed job must not leave provenance behind"
        );
        let (status, job_id): (String, Option<i64>) = conn
            .query_row(
                "SELECT status, job_id FROM mail_match_inbox WHERE id = ?1",
                params![inbox],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "pending");
        assert_eq!(job_id, None);

        // And it can be accepted again.
        assert!(accept_new(&mut conn, inbox).unwrap().created);
    }

    #[test]
    fn a_draft_without_a_company_still_becomes_a_job_the_user_can_fix() {
        let mut conn = db();
        let mut draft = full_draft();
        draft["company"] = json!("  ");
        let inbox = seed_inbox(&conn, draft);

        let outcome = accept_new(&mut conn, inbox).unwrap();

        let company: String = conn
            .query_row("SELECT company FROM jobs WHERE id = ?1", params![outcome.job_id], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(company, UNKNOWN_COMPANY);
    }

    #[test]
    fn an_accepted_job_keeps_the_board_link_next_to_the_employer_ad() {
        let mut conn = db();
        let mut draft = full_draft();
        draft["url"] = json!("https://candidate.hr-manager.net/ad/123");
        draft["board_url"] = json!("https://www.jobindex.dk/c?t=h1000001");
        let inbox = seed_inbox(&conn, draft);

        let outcome = accept_new(&mut conn, inbox).unwrap();

        let (url, board_url): (String, Option<String>) = conn
            .query_row(
                "SELECT url, board_url FROM jobs WHERE id = ?1",
                params![outcome.job_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(url, "https://candidate.hr-manager.net/ad/123");
        assert_eq!(
            board_url.as_deref(),
            Some("https://www.jobindex.dk/c?t=h1000001")
        );
        assert!(
            outcome.fields_written.iter().any(|f| f == "board_url"),
            "provenance must cover the board link: {:?}",
            outcome.fields_written
        );
    }

    // ----- Step 4: idempotency --------------------------------------------

    #[test]
    fn two_rapid_accepts_create_one_job() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, full_draft());

        let first = accept_new(&mut conn, inbox).unwrap();
        let second = accept_new(&mut conn, inbox).unwrap();

        assert!(first.created);
        assert!(!second.created, "the second accept must be a no-op");
        assert_eq!(second.job_id, first.job_id);

        let jobs: i64 = conn
            .query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(jobs, 1, "a double click must not create two jobs");
    }

    #[test]
    fn the_inbox_row_moves_to_accepted_and_links_the_job() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, full_draft());
        let outcome = accept_new(&mut conn, inbox).unwrap();

        let (status, job_id): (String, i64) = conn
            .query_row(
                "SELECT status, job_id FROM mail_match_inbox WHERE id = ?1",
                params![inbox],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "accepted");
        assert_eq!(job_id, outcome.job_id);
    }

    // ----- Step 5: provenance and mail_score ------------------------------

    #[test]
    fn accept_records_provenance_per_written_field() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, full_draft());
        let outcome = accept_new(&mut conn, inbox).unwrap();

        let mut stmt = conn
            .prepare("SELECT field, source, run_id FROM job_field_provenance WHERE job_id = ?1")
            .unwrap();
        let rows: Vec<(String, String, Option<String>)> = stmt
            .query_map(params![outcome.job_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        let fields: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
        assert!(fields.contains(&"deadline"), "{fields:?}");
        assert!(fields.contains(&"contact_name"), "{fields:?}");
        assert!(
            !fields.contains(&"contact_email"),
            "a field we did not write must not claim provenance: {fields:?}"
        );
        assert!(rows.iter().all(|r| r.1 == "mail_scan"));
        assert!(rows.iter().all(|r| r.2.as_deref() == Some("r1")));
    }

    #[test]
    fn accept_sets_mail_score_and_leaves_priority_alone() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, full_draft());
        let outcome = accept_new(&mut conn, inbox).unwrap();

        let (score, reason, scored_at, priority, status): (
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<i64>,
            String,
        ) = conn
            .query_row(
                "SELECT mail_score, mail_score_reason, mail_scored_at, priority, status
                 FROM jobs WHERE id = ?1",
                params![outcome.job_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();

        assert_eq!(score, Some(8));
        assert_eq!(reason.as_deref(), Some("good match"));
        assert!(scored_at.is_some());
        assert_eq!(priority, None, "a mail score is never a priority (spec §0.1)");
        assert_eq!(status, "Interesting");
    }

    #[test]
    fn a_scan_created_job_always_starts_as_interesting() {
        let mut conn = db();
        // Even if the draft claims something else.
        let mut draft = full_draft();
        draft["status"] = json!("Offer");
        draft["priority"] = json!(1);
        let inbox = seed_inbox(&conn, draft);

        let outcome = accept_new(&mut conn, inbox).unwrap();

        let (status, priority): (String, Option<i64>) = conn
            .query_row(
                "SELECT status, priority FROM jobs WHERE id = ?1",
                params![outcome.job_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "Interesting");
        assert_eq!(priority, None);
    }

    #[test]
    fn a_scan_never_claims_the_workflow_fields() {
        for protected in ["status", "priority", "id", "company", "created_at"] {
            assert!(
                !DRAFT_FIELDS.contains(&protected),
                "{protected} must not be written from a Draft"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Create a Job from a match's stored Draft in one click. The inbox offers Undo
/// right after, which is the human check the old prefilled form used to be.
#[tauri::command]
pub fn mail_match_accept_new(
    app: tauri::AppHandle,
    inbox_id: i64,
) -> Result<AcceptOutcome, String> {
    let mut conn = crate::db::connection(&app)?;
    accept_new(&mut conn, inbox_id)
}

/// Undo a one-click accept: the Job goes, the match returns to the inbox.
#[tauri::command]
pub fn mail_match_undo_accept(app: tauri::AppHandle, inbox_id: i64) -> Result<(), String> {
    let mut conn = crate::db::connection(&app)?;
    undo_accept(&mut conn, inbox_id)
}
