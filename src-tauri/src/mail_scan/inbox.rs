//! Read and act on the Mail Match Inbox (spec §11.1).
//!
//! Its own surface, separate from the Capture Inbox (ADR 0003). Everything here is
//! either a query or a reversible action — the one irreversible step, creating a Job,
//! lives in [`super::accept`], one click with an Undo right after.

use rusqlite::{params, Connection, OptionalExtension};

use crate::mail_scan::cluster::{dismiss, restore};
use crate::mail_scan::enrichment::is_snippet_only;
use crate::mail_scan::status::{EnrichmentState, InboxStatus, RunStatus, ScoreState};

/// One row as the inbox list renders it.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMatchRow {
    pub id: i64,
    pub fingerprint_id: String,
    pub status: InboxStatus,
    pub job_id: Option<i64>,
    /// `None` when `scoreState` is `invalid` — the UI shows `?`, never a number.
    pub score: Option<i64>,
    pub score_reason: Option<String>,
    pub score_state: ScoreState,
    pub suspicious: bool,
    pub near_duplicate_of: Option<String>,
    pub enrichment_state: EnrichmentState,
    pub enrichment_error: Option<String>,
    /// The board never serves its listing page (Indeed), so the mail snippet is all
    /// there is. Expected, and shown calmer than an incomplete enrichment.
    pub snippet_only: bool,
    pub draft_json: String,
    pub source_board: Option<String>,
    pub message_date: Option<String>,
    pub listing_url: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    /// How many times this listing has been seen across runs.
    pub seen_count: i64,
    pub last_seen_at: String,
    pub updated_at: String,
}

const ROW_COLUMNS: &str = "i.id, i.fingerprint_id, i.status, i.job_id, i.score,
     i.score_reason, i.score_state, i.suspicious, i.near_duplicate_of, i.enrichment_state,
     i.enrichment_error, i.draft_json, i.source_board, i.message_date, i.listing_url,
     i.title, i.company, f.seen_count, f.last_seen_at, i.updated_at";

fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<MailMatchRow> {
    let enrichment_state: EnrichmentState = r.get(9)?;
    let enrichment_error: Option<String> = r.get(10)?;
    Ok(MailMatchRow {
        id: r.get(0)?,
        fingerprint_id: r.get(1)?,
        status: r.get(2)?,
        job_id: r.get(3)?,
        score: r.get(4)?,
        score_reason: r.get(5)?,
        score_state: r.get(6)?,
        suspicious: r.get::<_, i64>(7)? != 0,
        near_duplicate_of: r.get(8)?,
        snippet_only: is_snippet_only(enrichment_state, enrichment_error.as_deref()),
        enrichment_state,
        enrichment_error,
        draft_json: r.get(11)?,
        source_board: r.get(12)?,
        message_date: r.get(13)?,
        listing_url: r.get(14)?,
        title: r.get(15)?,
        company: r.get(16)?,
        seen_count: r.get(17)?,
        last_seen_at: r.get(18)?,
        updated_at: r.get(19)?,
    })
}

/// Rows for one status, highest score first then most recent (spec §11.1).
///
/// `NULLS LAST` matters: an `invalid` score is stored as NULL, and sorting it to the
/// top would put the rows we trust least in front of the ones we trust most.
pub fn list_rows(conn: &Connection, status: InboxStatus) -> Result<Vec<MailMatchRow>, String> {
    let sql = format!(
        "SELECT {ROW_COLUMNS}
         FROM mail_match_inbox i
         JOIN mail_fingerprints f ON f.fingerprint_id = i.fingerprint_id
         WHERE i.status = ?1
         ORDER BY i.score DESC NULLS LAST, f.last_seen_at DESC, i.id DESC"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![status], map_row)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// A dismissal, with enough context to decide whether restoring it is right.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DismissedRow {
    pub fingerprint_id: String,
    pub reason: Option<String>,
    pub dismissed_at: String,
    pub dismissed_run: Option<String>,
    pub title: Option<String>,
    pub company: Option<String>,
    pub listing_url: Option<String>,
}

/// Everything currently suppressed. A dismissal the user cannot see or undo is the
/// failure this view exists to prevent (spec §5.3).
pub fn list_dismissed(conn: &Connection) -> Result<Vec<DismissedRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT d.fingerprint_id, d.reason, d.dismissed_at, d.dismissed_run,
                    i.title, i.company, i.listing_url
             FROM mail_match_dismissals d
             LEFT JOIN mail_match_inbox i ON i.fingerprint_id = d.fingerprint_id
             GROUP BY d.fingerprint_id
             ORDER BY d.dismissed_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(DismissedRow {
                fingerprint_id: r.get(0)?,
                reason: r.get(1)?,
                dismissed_at: r.get(2)?,
                dismissed_run: r.get(3)?,
                title: r.get(4)?,
                company: r.get(5)?,
                listing_url: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// A finished run, rendered by the same component as a live one (spec §8.4).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRow {
    pub run_id: String,
    pub status: RunStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    /// Raw `stats_json`; the frontend parses it with the same reducer it uses for
    /// live progress events, so History and a running scan cannot diverge.
    pub stats_json: String,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
    pub model_id: Option<String>,
}

pub fn list_runs(conn: &Connection, limit: i64) -> Result<Vec<RunRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT run_id, status, started_at, finished_at, stats_json,
                    error_code, error_summary, model_id
             FROM mail_scan_runs ORDER BY started_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(RunRow {
                run_id: r.get(0)?,
                status: r.get(1)?,
                started_at: r.get(2)?,
                finished_at: r.get(3)?,
                stats_json: r.get(4)?,
                error_code: r.get(5)?,
                error_summary: r.get(6)?,
                model_id: r.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// Dismiss the fingerprint behind an inbox row and retire the row.
///
/// Both happen together: a dismissal that suppressed future scans but left the row
/// on screen would look broken, and one that hid the row without recording the
/// dismissal would come back next run.
pub fn dismiss_row(conn: &mut Connection, inbox_id: i64, reason: Option<&str>) -> Result<(), String> {
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT fingerprint_id, last_run_id FROM mail_match_inbox WHERE id = ?1",
            params![inbox_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((fingerprint_id, run_id)) = row else {
        return Err("That inbox row no longer exists.".into());
    };

    dismiss(conn, &fingerprint_id, run_id.as_deref(), reason)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE mail_match_inbox SET status = ?3, updated_at = ?1
         WHERE id = ?2 AND status = ?4",
        params![now, inbox_id, InboxStatus::Dismissed, InboxStatus::Pending],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Undo a dismissal and re-admit any row it suppressed this run (spec §5.3).
pub fn restore_fingerprint(conn: &mut Connection, fingerprint_id: &str) -> Result<(), String> {
    restore(conn, fingerprint_id)?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE mail_match_inbox SET status = ?3, updated_at = ?1
         WHERE fingerprint_id = ?2 AND status = ?4",
        params![now, fingerprint_id, InboxStatus::Pending, InboxStatus::Dismissed],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn mail_match_list(
    app: tauri::AppHandle,
    status: Option<InboxStatus>,
) -> Result<Vec<MailMatchRow>, String> {
    let conn = crate::db::connection(&app)?;
    list_rows(&conn, status.unwrap_or(InboxStatus::Pending))
}

#[tauri::command]
pub fn mail_match_list_dismissed(app: tauri::AppHandle) -> Result<Vec<DismissedRow>, String> {
    let conn = crate::db::connection(&app)?;
    list_dismissed(&conn)
}

#[tauri::command]
pub fn mail_scan_list_runs(app: tauri::AppHandle, limit: Option<i64>) -> Result<Vec<RunRow>, String> {
    let conn = crate::db::connection(&app)?;
    list_runs(&conn, limit.unwrap_or(20))
}

#[tauri::command]
pub fn mail_match_dismiss(
    app: tauri::AppHandle,
    inbox_id: i64,
    reason: Option<String>,
) -> Result<(), String> {
    let mut conn = crate::db::connection(&app)?;
    dismiss_row(&mut conn, inbox_id, reason.as_deref())
}

#[tauri::command]
pub fn mail_match_restore(app: tauri::AppHandle, fingerprint_id: String) -> Result<(), String> {
    let mut conn = crate::db::connection(&app)?;
    restore_fingerprint(&mut conn, &fingerprint_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO mail_scan_runs (run_id, status, started_at, stats_json)
             VALUES ('r1','completed','2026-09-09T00:00:00Z','{\"inboxNew\":2}')",
            [],
        )
        .unwrap();
        conn
    }

    fn seed(conn: &Connection, fp: &str, score: Option<i64>, state: &str, seen: i64, last: &str) -> i64 {
        conn.execute(
            "INSERT INTO mail_fingerprints (fingerprint_id, strong_key, weak_key, first_seen_at, last_seen_at, seen_count)
             VALUES (?1, ?1, ?1, 't', ?2, ?3)",
            params![fp, last, seen],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mail_match_inbox (
                fingerprint_id, kind, status, score, score_reason, score_state,
                draft_json, enrichment_state, source_board,
                first_run_id, last_run_id, created_at, updated_at
             ) VALUES (?1, 'new', 'pending', ?2, 'reason', ?3,
                       '{\"title\":\"T\",\"company\":\"C\"}', 'complete', 'indeed',
                       'r1','r1','t', ?4)",
            params![fp, score, state, last],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn pending_sorts_by_score_then_recency() {
        let conn = db();
        seed(&conn, "fp-low", Some(4), "ok", 1, "2026-09-09T10:00:00Z");
        seed(&conn, "fp-high-old", Some(9), "ok", 1, "2026-09-01T10:00:00Z");
        seed(&conn, "fp-high-new", Some(9), "ok", 1, "2026-09-09T10:00:00Z");

        let rows = list_rows(&conn, InboxStatus::Pending).unwrap();
        let order: Vec<&str> = rows.iter().map(|r| r.fingerprint_id.as_str()).collect();

        assert_eq!(order, vec!["fp-high-new", "fp-high-old", "fp-low"]);
    }

    #[test]
    fn an_invalid_score_sorts_last_rather_than_first() {
        // NULL sorting to the top would put the rows we trust least in front.
        let conn = db();
        seed(&conn, "fp-ok", Some(5), "ok", 1, "2026-09-01T00:00:00Z");
        seed(&conn, "fp-invalid", None, "invalid", 1, "2026-09-09T00:00:00Z");

        let rows = list_rows(&conn, InboxStatus::Pending).unwrap();
        assert_eq!(rows[0].fingerprint_id, "fp-ok");
        assert_eq!(rows[1].score, None);
        assert_eq!(rows[1].score_state, ScoreState::Invalid);
    }

    #[test]
    fn rows_carry_the_seen_count_from_the_fingerprint() {
        let conn = db();
        seed(&conn, "fp-seen", Some(8), "ok", 3, "2026-09-09T00:00:00Z");
        let rows = list_rows(&conn, InboxStatus::Pending).unwrap();
        assert_eq!(rows[0].seen_count, 3);
    }

    fn set_enrichment(conn: &Connection, id: i64, state: &str, error: Option<&str>) {
        conn.execute(
            "UPDATE mail_match_inbox SET enrichment_state = ?2, enrichment_error = ?3 WHERE id = ?1",
            params![id, state, error],
        )
        .unwrap();
    }

    #[test]
    fn a_listing_whose_board_blocks_fetching_is_marked_snippet_only() {
        // Indeed never serves its page to the app: expected, not a failure to flag.
        let conn = db();
        let id = seed(&conn, "fp-indeed", Some(8), "ok", 1, "2026-09-09T00:00:00Z");
        set_enrichment(
            &conn,
            id,
            "skipped",
            Some(crate::mail_scan::enrichment::NOT_FETCHABLE),
        );

        let rows = list_rows(&conn, InboxStatus::Pending).unwrap();

        assert!(rows[0].snippet_only);
    }

    #[test]
    fn a_skipped_or_failed_fetch_is_not_snippet_only() {
        // Budget ran out, or the fetch broke: those really are incomplete.
        let conn = db();
        let skipped = seed(&conn, "fp-budget", Some(8), "ok", 1, "2026-09-09T00:00:00Z");
        set_enrichment(&conn, skipped, "skipped", None);
        let failed = seed(&conn, "fp-broken", Some(7), "ok", 1, "2026-09-09T00:00:00Z");
        set_enrichment(&conn, failed, "failed", Some("HTTP 500"));

        let rows = list_rows(&conn, InboxStatus::Pending).unwrap();

        assert!(rows.iter().all(|r| !r.snippet_only), "{rows:?}");
    }

    #[test]
    fn dismissing_a_row_hides_it_and_records_the_dismissal() {
        let mut conn = db();
        let id = seed(&conn, "fp-noise", Some(8), "ok", 1, "2026-09-09T00:00:00Z");

        dismiss_row(&mut conn, id, Some("recruiter spam")).unwrap();

        assert!(list_rows(&conn, InboxStatus::Pending).unwrap().is_empty());
        let dismissed = list_dismissed(&conn).unwrap();
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].reason.as_deref(), Some("recruiter spam"));
        assert_eq!(dismissed[0].dismissed_run.as_deref(), Some("r1"));
    }

    #[test]
    fn restoring_readmits_the_row_immediately() {
        let mut conn = db();
        let id = seed(&conn, "fp-oops", Some(8), "ok", 1, "2026-09-09T00:00:00Z");
        dismiss_row(&mut conn, id, None).unwrap();

        restore_fingerprint(&mut conn, "fp-oops").unwrap();

        let rows = list_rows(&conn, InboxStatus::Pending).unwrap();
        assert_eq!(rows.len(), 1, "restore must bring the row back without a rescan");
        assert!(list_dismissed(&conn).unwrap().is_empty());
    }

    #[test]
    fn dismissing_one_listing_leaves_the_others_alone() {
        let mut conn = db();
        let a = seed(&conn, "fp-a", Some(8), "ok", 1, "2026-09-09T00:00:00Z");
        seed(&conn, "fp-b", Some(8), "ok", 1, "2026-09-09T00:00:00Z");

        dismiss_row(&mut conn, a, None).unwrap();

        let rows = list_rows(&conn, InboxStatus::Pending).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].fingerprint_id, "fp-b");
    }

    #[test]
    fn history_exposes_stats_json_for_the_shared_renderer() {
        let conn = db();
        let runs = list_runs(&conn, 10).unwrap();
        assert_eq!(runs.len(), 1);
        assert!(runs[0].stats_json.contains("inboxNew"));
        assert_eq!(runs[0].status, RunStatus::Completed);
    }
}
