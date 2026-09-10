//! Accepting a match into the Jobs table (spec §5.2, §5.6).
//!
//! Three properties this module exists to hold, each of which is a silent data-loss
//! bug if it breaks:
//!
//! 1. **The accept-time re-diff.** A suggestion is computed against the job as it was
//!    when the scan ran. If the user edited that job in between, applying the stored
//!    patch would overwrite their own work with a month-old guess from an email. The
//!    patch is therefore recomputed against the live row at accept time, and the UI is
//!    told the job changed.
//! 2. **Idempotency.** Accept moves the row `pending → accepted` in the same
//!    transaction as the job write, guarded by `WHERE status = 'pending'`. A double
//!    click cannot create two Jobs.
//! 3. **Provenance.** Every field written from a scan gets a `job_field_provenance`
//!    row, so "where did this deadline come from?" has an answer.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

/// Fields a mail scan may ever write onto an existing Job.
///
/// `status` and `priority` are absent and must stay absent: they are the user's
/// workflow state, and nothing derived from an email gets to move a job through it
/// (spec §0.1, §5.2).
pub const PATCHABLE_FIELDS: &[&str] = &[
    "title",
    "url",
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

/// Never writable from a scan, at any point in this module.
pub const PROTECTED_FIELDS: &[&str] = &["status", "priority", "id", "company", "created_at"];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldSuggestion {
    pub field: String,
    pub suggested: String,
    /// What the live Job holds right now; `None` when the field is blank.
    pub current: Option<String>,
    /// False when the live Job already has a value — the patch skips it.
    pub applicable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreview {
    pub inbox_id: i64,
    pub job_id: i64,
    /// True when the Job was edited after the scan computed this suggestion.
    pub job_changed_since_scan: bool,
    pub fields: Vec<FieldSuggestion>,
}

impl UpdatePreview {
    /// How many fields the patch would actually write. Tests assert on it; the UI
    /// reads `fields` directly so it can show the skipped ones too.
    #[cfg(test)]
    pub fn applicable_count(&self) -> usize {
        self.fields.iter().filter(|f| f.applicable).count()
    }
}

fn is_blank(v: Option<&String>) -> bool {
    v.map(|s| s.trim().is_empty()).unwrap_or(true)
}

/// A Job's patchable columns, plus its `updated_at` — the baseline the re-diff
/// compares against.
pub type JobSnapshot = (HashMap<String, String>, String);

/// Read the patchable columns of one Job.
pub fn read_job_fields(conn: &Connection, job_id: i64) -> Result<Option<JobSnapshot>, String> {
    let cols = PATCHABLE_FIELDS.join(", ");
    let sql = format!("SELECT {cols}, updated_at FROM jobs WHERE id = ?1");
    conn.query_row(&sql, params![job_id], |row| {
        let mut map = HashMap::new();
        for (i, field) in PATCHABLE_FIELDS.iter().enumerate() {
            if let Some(v) = row.get::<_, Option<String>>(i)? {
                map.insert((*field).to_string(), v);
            }
        }
        let updated_at: String = row.get(PATCHABLE_FIELDS.len())?;
        Ok((map, updated_at))
    })
    .optional()
    .map_err(|e| e.to_string())
}

/// Recompute the effective patch against the **live** Job (spec §5.6 step 2).
///
/// Only fields that are still blank are kept. A suggestion never overwrites a value
/// the user typed, whether they typed it before the scan or after it.
pub fn recompute_patch(
    live: &HashMap<String, String>,
    suggested: &HashMap<String, Value>,
) -> Vec<FieldSuggestion> {
    let mut out = Vec::new();
    for field in PATCHABLE_FIELDS {
        let Some(raw) = suggested.get(*field) else {
            continue;
        };
        let Some(text) = raw.as_str().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let current = live.get(*field).cloned();
        let applicable = is_blank(current.as_ref());
        out.push(FieldSuggestion {
            field: (*field).to_string(),
            suggested: text.to_string(),
            current: current.filter(|c| !c.trim().is_empty()),
            applicable,
        });
    }
    out
}

/// Build the preview shown before an update is applied.
pub fn preview_update(conn: &Connection, inbox_id: i64) -> Result<UpdatePreview, String> {
    let row: (Option<i64>, String, Option<String>, String) = conn
        .query_row(
            "SELECT job_id, draft_json, base_job_updated_at, status
             FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "That inbox row no longer exists.".to_string())?;

    let (job_id, draft_json, base_updated_at, status) = row;
    if status != "pending" {
        return Err(format!("That suggestion is already {status}."));
    }
    let job_id = job_id.ok_or_else(|| "That suggestion is not linked to a job.".to_string())?;

    let (live, live_updated_at) = read_job_fields(conn, job_id)?
        .ok_or_else(|| "The job this suggestion refers to has been deleted.".to_string())?;

    let suggested: HashMap<String, Value> = serde_json::from_str(&draft_json)
        .map_err(|e| format!("Could not read the stored suggestion: {e}"))?;

    Ok(UpdatePreview {
        inbox_id,
        job_id,
        // Absent baseline means we cannot prove it did not change; say so rather than
        // implying the suggestion is still fresh.
        job_changed_since_scan: base_updated_at
            .map(|base| base != live_updated_at)
            .unwrap_or(true),
        fields: recompute_patch(&live, &suggested),
    })
}

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
             SET status = 'accepted', job_id = ?1, updated_at = ?2
             WHERE id = ?3 AND status = 'pending'",
            params![job_id, now, inbox_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(changed == 1)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptOutcome {
    pub job_id: i64,
    /// Fields actually written. Empty on an update whose fields were all filled in
    /// by the user since the scan.
    pub fields_written: Vec<String>,
    /// True when this call did the work; false when it lost the race to a double-click.
    pub created: bool,
}

/// Apply an update suggestion to an existing Job.
///
/// The patch is recomputed here, inside the transaction, rather than trusting whatever
/// the UI last rendered.
pub fn accept_update(conn: &mut Connection, inbox_id: i64) -> Result<AcceptOutcome, String> {
    let preview = preview_update(conn, inbox_id)?;
    let now = chrono::Utc::now().to_rfc3339();
    let run_id: Option<String> = conn
        .query_row(
            "SELECT last_run_id FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Re-read inside the transaction: `preview_update` ran outside it.
    let (live, _) = read_job_fields(&tx, preview.job_id)?
        .ok_or_else(|| "The job this suggestion refers to has been deleted.".to_string())?;

    let mut written = Vec::new();
    for suggestion in &preview.fields {
        // Recheck blankness against the row we hold the transaction on.
        if !is_blank(live.get(&suggestion.field)) {
            continue;
        }
        debug_assert!(
            !PROTECTED_FIELDS.contains(&suggestion.field.as_str()),
            "PATCHABLE_FIELDS must never overlap PROTECTED_FIELDS"
        );
        tx.execute(
            &format!("UPDATE jobs SET {} = ?1 WHERE id = ?2", suggestion.field),
            params![&suggestion.suggested, preview.job_id],
        )
        .map_err(|e| e.to_string())?;
        record_provenance(
            &tx,
            preview.job_id,
            &suggestion.field,
            run_id.as_deref(),
            &now,
        )?;
        written.push(suggestion.field.clone());
    }

    if !claim_pending(&tx, inbox_id, preview.job_id, &now)? {
        // Somebody else accepted it between the preview and here.
        tx.rollback().map_err(|e| e.to_string())?;
        return Ok(AcceptOutcome {
            job_id: preview.job_id,
            fields_written: Vec::new(),
            created: false,
        });
    }

    // Refresh the advisory score and the row's own timestamp. `status` and `priority`
    // are deliberately untouched.
    apply_mail_score(&tx, preview.job_id, inbox_id, &now)?;
    tx.execute(
        "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
        params![&now, preview.job_id],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(AcceptOutcome {
        job_id: preview.job_id,
        fields_written: written,
        created: true,
    })
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

/// Create a Job from a `new` match, using the payload the user approved in the form.
///
/// The inbox row is claimed in the same transaction as the insert, so two rapid
/// accepts produce one Job and one `accepted` row.
pub fn accept_new(
    conn: &mut Connection,
    inbox_id: i64,
    payload: &crate::db::NewJob,
) -> Result<AcceptOutcome, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let run_id: Option<String> = conn
        .query_row(
            "SELECT last_run_id FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Claim first: if this loses the race, no Job is created at all.
    let already: Option<(String, Option<i64>)> = tx
        .query_row(
            "SELECT status, job_id FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((status, existing_job)) = already else {
        return Err("That inbox row no longer exists.".into());
    };
    if status != "pending" {
        tx.rollback().map_err(|e| e.to_string())?;
        return Ok(AcceptOutcome {
            job_id: existing_job.unwrap_or_default(),
            fields_written: Vec::new(),
            created: false,
        });
    }

    // A scan-created Job always starts in `Interesting` (spec §5.2); the score is
    // advisory and does not move it further.
    let mut to_insert = payload.clone();
    to_insert.status = "Interesting".to_string();
    to_insert.priority = None;

    let job_id = crate::db::insert_job_returning_id(&tx, &to_insert, &now)?;

    let mut written = Vec::new();
    for field in PATCHABLE_FIELDS {
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

fn payload_field_is_set(payload: &crate::db::NewJob, field: &str) -> bool {
    let get = |v: &Option<String>| v.as_deref().map(str::trim).is_some_and(|s| !s.is_empty());
    match field {
        "title" => get(&payload.title),
        "url" => get(&payload.url),
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
    use crate::db::NewJob;
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

    fn new_job(company: &str) -> NewJob {
        NewJob {
            company: company.into(),
            title: Some("Rust Engineer".into()),
            url: Some("https://example.com/job".into()),
            raw_text: None,
            status: "Interesting".into(),
            deadline: Some("2026-10-01".into()),
            interview_date: None,
            start_date: None,
            tags: None,
            detected_language: None,
            notes: None,
            contact_name: Some("Ada".into()),
            contact_email: None,
            contact_phone: None,
            workplace_street: None,
            workplace_city: Some("København".into()),
            workplace_postal_code: None,
            work_mode: Some("Hybrid".into()),
            salary_range: None,
            contract_type: None,
            priority: None,
            reference_number: None,
            source: Some("indeed".into()),
        }
    }

    /// Insert a Job and return its id and `updated_at`.
    fn seed_job(conn: &Connection, company: &str, deadline: Option<&str>) -> (i64, String) {
        conn.execute(
            "INSERT INTO jobs (company, title, status, deadline, priority, created_at, updated_at)
             VALUES (?1, 'Existing title', 'Applied', ?2, 3, 'T0', 'T0')",
            params![company, deadline],
        )
        .unwrap();
        let id = conn.last_insert_rowid();
        (id, "T0".to_string())
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_inbox(
        conn: &Connection,
        kind: &str,
        job_id: Option<i64>,
        draft: serde_json::Value,
        base_updated_at: Option<&str>,
    ) -> i64 {
        conn.execute(
            "INSERT INTO mail_match_inbox (
                fingerprint_id, kind, status, job_id, score, score_reason, score_state,
                draft_json, enrichment_state, base_job_updated_at,
                first_run_id, last_run_id, created_at, updated_at
             ) VALUES ('fp1', ?1, 'pending', ?2, 8, 'good match', 'ok',
                       ?3, 'complete', ?4, 'r1','r1','t','t')",
            params![kind, job_id, draft.to_string(), base_updated_at],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    // ----- Step 3: the accept-time re-diff --------------------------------

    #[test]
    fn a_suggestion_never_overwrites_a_field_the_user_edited_after_the_scan() {
        // The scan saw a blank deadline and suggested one. Between the scan and the
        // accept, the user typed their own. A blind patch would silently replace it.
        let mut conn = db();
        let (job_id, base) = seed_job(&conn, "Acme", None);
        let inbox = seed_inbox(
            &conn,
            "update_suggestion",
            Some(job_id),
            json!({ "deadline": "2026-10-01", "work_mode": "Hybrid" }),
            Some(&base),
        );

        conn.execute(
            "UPDATE jobs SET deadline = '2026-12-24', updated_at = 'T1' WHERE id = ?1",
            params![job_id],
        )
        .unwrap();

        let preview = preview_update(&conn, inbox).unwrap();
        assert!(
            preview.job_changed_since_scan,
            "the UI must be able to say the job changed since the scan"
        );
        let deadline = preview.fields.iter().find(|f| f.field == "deadline").unwrap();
        assert!(!deadline.applicable, "a filled field must be skipped");
        assert_eq!(deadline.current.as_deref(), Some("2026-12-24"));

        let outcome = accept_update(&mut conn, inbox).unwrap();
        assert_eq!(outcome.fields_written, vec!["work_mode".to_string()]);

        let live: String = conn
            .query_row("SELECT deadline FROM jobs WHERE id = ?1", params![job_id], |r| r.get(0))
            .unwrap();
        assert_eq!(live, "2026-12-24", "the user's own edit must survive");
    }

    #[test]
    fn an_unchanged_job_is_not_reported_as_changed() {
        let mut conn = db();
        let (job_id, base) = seed_job(&conn, "Acme", None);
        let inbox = seed_inbox(
            &conn,
            "update_suggestion",
            Some(job_id),
            json!({ "deadline": "2026-10-01" }),
            Some(&base),
        );

        let preview = preview_update(&conn, inbox).unwrap();
        assert!(!preview.job_changed_since_scan);
        assert_eq!(preview.applicable_count(), 1);

        let outcome = accept_update(&mut conn, inbox).unwrap();
        assert_eq!(outcome.fields_written, vec!["deadline".to_string()]);
    }

    #[test]
    fn a_missing_baseline_is_reported_as_changed_rather_than_assumed_fresh() {
        let conn = db();
        let (job_id, _) = seed_job(&conn, "Acme", None);
        let inbox = seed_inbox(
            &conn,
            "update_suggestion",
            Some(job_id),
            json!({ "deadline": "2026-10-01" }),
            None,
        );
        assert!(preview_update(&conn, inbox).unwrap().job_changed_since_scan);
    }

    #[test]
    fn recompute_patch_keeps_only_blank_fields() {
        let live: HashMap<String, String> = [
            ("deadline".to_string(), "2026-12-24".to_string()),
            ("work_mode".to_string(), "  ".to_string()),
        ]
        .into_iter()
        .collect();
        let suggested: HashMap<String, Value> = [
            ("deadline".to_string(), json!("2026-10-01")),
            ("work_mode".to_string(), json!("Remote")),
            ("salary_range".to_string(), json!("60k")),
        ]
        .into_iter()
        .collect();

        let patch = recompute_patch(&live, &suggested);
        let applicable: Vec<&str> = patch
            .iter()
            .filter(|f| f.applicable)
            .map(|f| f.field.as_str())
            .collect();

        assert!(applicable.contains(&"work_mode"), "whitespace counts as blank");
        assert!(applicable.contains(&"salary_range"));
        assert!(!applicable.contains(&"deadline"));
    }

    #[test]
    fn a_suggestion_can_never_touch_status_or_priority() {
        let mut conn = db();
        let (job_id, base) = seed_job(&conn, "Acme", None);
        let inbox = seed_inbox(
            &conn,
            "update_suggestion",
            Some(job_id),
            // A hostile or confused suggestion naming workflow fields.
            json!({ "deadline": "2026-10-01", "status": "Rejected", "priority": 1 }),
            Some(&base),
        );

        accept_update(&mut conn, inbox).unwrap();

        let (status, priority): (String, i64) = conn
            .query_row(
                "SELECT status, priority FROM jobs WHERE id = ?1",
                params![job_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, "Applied", "a scan must not move a job through the workflow");
        assert_eq!(priority, 3, "priority is the user's, never a score's");
    }

    // ----- Step 4: idempotency --------------------------------------------

    #[test]
    fn two_rapid_accepts_create_one_job() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, "new", None, json!({ "company": "Acme" }), None);

        let first = accept_new(&mut conn, inbox, &new_job("Acme")).unwrap();
        let second = accept_new(&mut conn, inbox, &new_job("Acme")).unwrap();

        assert!(first.created);
        assert!(!second.created, "the second accept must be a no-op");
        assert_eq!(second.job_id, first.job_id);

        let jobs: i64 = conn
            .query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(jobs, 1, "a double click must not create two jobs");
    }

    #[test]
    fn accepting_an_update_twice_applies_it_once() {
        let mut conn = db();
        let (job_id, base) = seed_job(&conn, "Acme", None);
        let inbox = seed_inbox(
            &conn,
            "update_suggestion",
            Some(job_id),
            json!({ "deadline": "2026-10-01" }),
            Some(&base),
        );

        assert!(accept_update(&mut conn, inbox).unwrap().created);
        let err = accept_update(&mut conn, inbox).unwrap_err();
        assert!(err.contains("already accepted"), "{err}");

        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_match_inbox WHERE status = 'accepted'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 1);
    }

    #[test]
    fn the_inbox_row_moves_to_accepted_and_links_the_job() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, "new", None, json!({ "company": "Acme" }), None);
        let outcome = accept_new(&mut conn, inbox, &new_job("Acme")).unwrap();

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
        let inbox = seed_inbox(&conn, "new", None, json!({ "company": "Acme" }), None);
        let outcome = accept_new(&mut conn, inbox, &new_job("Acme")).unwrap();

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
    fn update_records_provenance_only_for_fields_it_actually_wrote() {
        let mut conn = db();
        let (job_id, base) = seed_job(&conn, "Acme", Some("2026-11-11"));
        let inbox = seed_inbox(
            &conn,
            "update_suggestion",
            Some(job_id),
            json!({ "deadline": "2026-10-01", "salary_range": "60-70k" }),
            Some(&base),
        );

        accept_update(&mut conn, inbox).unwrap();

        let fields: Vec<String> = conn
            .prepare("SELECT field FROM job_field_provenance WHERE job_id = ?1")
            .unwrap()
            .query_map(params![job_id], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(fields, vec!["salary_range".to_string()]);
    }

    #[test]
    fn accept_sets_mail_score_and_leaves_priority_alone() {
        let mut conn = db();
        let inbox = seed_inbox(&conn, "new", None, json!({ "company": "Acme" }), None);
        let outcome = accept_new(&mut conn, inbox, &new_job("Acme")).unwrap();

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
        let inbox = seed_inbox(&conn, "new", None, json!({ "company": "Acme" }), None);
        // Even if the caller asks for something else.
        let mut payload = new_job("Acme");
        payload.status = "Offer".into();
        payload.priority = Some(1);

        let outcome = accept_new(&mut conn, inbox, &payload).unwrap();

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
    fn patchable_and_protected_fields_never_overlap() {
        // The guard that makes the SQL interpolation in `accept_update` safe: only
        // names from this constant are ever formatted into a statement.
        for protected in PROTECTED_FIELDS {
            assert!(
                !PATCHABLE_FIELDS.contains(protected),
                "{protected} must not be patchable"
            );
        }
        for field in PATCHABLE_FIELDS {
            assert!(
                field.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{field} is interpolated into SQL and must be a plain identifier"
            );
        }
    }

    #[test]
    fn deleting_a_job_takes_its_pending_suggestions_with_it() {
        // `mail_match_inbox.job_id` cascades, so a deleted job cannot leave behind a
        // suggestion pointing at nothing. Accepting one must fail cleanly either way.
        let conn = db();
        let (job_id, base) = seed_job(&conn, "Acme", None);
        let inbox = seed_inbox(
            &conn,
            "update_suggestion",
            Some(job_id),
            json!({ "deadline": "2026-10-01" }),
            Some(&base),
        );
        conn.execute("DELETE FROM jobs WHERE id = ?1", params![job_id]).unwrap();

        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_match_inbox WHERE id = ?1",
                params![inbox],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0, "the FK cascade must clean up the suggestion");

        let err = preview_update(&conn, inbox).unwrap_err();
        assert!(err.contains("no longer exists"), "{err}");
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Recomputed diff for an Update Suggestion, including the "job changed" state.
#[tauri::command]
pub fn mail_match_preview_update(
    app: tauri::AppHandle,
    inbox_id: i64,
) -> Result<UpdatePreview, String> {
    let conn = crate::db::connection(&app)?;
    preview_update(&conn, inbox_id)
}

/// Apply an Update Suggestion. The patch is recomputed against the live Job inside the
/// transaction, so what the UI last rendered cannot overwrite a newer edit.
#[tauri::command]
pub fn mail_match_accept_update(
    app: tauri::AppHandle,
    inbox_id: i64,
) -> Result<AcceptOutcome, String> {
    let mut conn = crate::db::connection(&app)?;
    accept_update(&mut conn, inbox_id)
}

/// Create a Job from a `new` match.
///
/// `payload` is what the user approved in the prefilled form — the draft is a prefill,
/// never a silent write (spec non-goal 4).
#[tauri::command]
pub fn mail_match_accept_new(
    app: tauri::AppHandle,
    inbox_id: i64,
    payload: crate::db::NewJob,
) -> Result<AcceptOutcome, String> {
    let mut conn = crate::db::connection(&app)?;
    accept_new(&mut conn, inbox_id, &payload)
}
