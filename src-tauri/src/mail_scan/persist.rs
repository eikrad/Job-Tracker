//! Per-item persistence for mail scan listings (spec §7.3).
//!
//! One transaction per listing, never one per run: a crash at listing 60 of 118 leaves
//! 59 usable rows and a `failed` run, and the score cache makes the redo nearly free.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::mail_scan::cluster::{is_dismissed, upsert_cluster};
use crate::mail_scan::enrichment::Enrichment;
use crate::mail_scan::protocol::ListingEvent;
use crate::mail_scan::score_cache::{record_sighting, ScoreIdentity};
use crate::mail_scan::scoring::ScoreOutcome;
use crate::mail_scan::status::{InboxStatus, RunStatus, SourceKind, Verdict};

/// Counters mirrored into `mail_scan_runs.stats_json` so the History view can render a
/// finished run through the same component as a live one (spec §8.4).
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStats {
    pub listings_committed: u32,
    pub messages_seen: u32,
    pub messages_parsed: u32,
    pub suppressed_by_dismissal: u32,
    pub under_cutoff: u32,
    pub inbox_new: u32,
    /// Listings that are already Jobs on the board, recorded and kept out of the inbox.
    pub already_tracked: u32,
    pub llm_calls: u32,
    pub enrichment_failures: u32,
    pub errors: u32,
    /// Run hit the call cap. Not a failure — the run still `completed`.
    pub budget_exhausted: bool,
}

impl RunStats {
    /// Count one listing's outcome. A dismissed listing is suppressed, not committed:
    /// the summary reports it separately so the user can find it in the Dismissed tab.
    pub fn count(&mut self, outcome: PersistOutcome) {
        match outcome {
            PersistOutcome::Committed => {
                self.listings_committed += 1;
                self.inbox_new += 1;
            }
            PersistOutcome::UnderCutoff => {
                self.listings_committed += 1;
                self.under_cutoff += 1;
            }
            PersistOutcome::SuppressedByDismissal => self.suppressed_by_dismissal += 1,
            PersistOutcome::AlreadyTracked => {
                self.listings_committed += 1;
                self.already_tracked += 1;
            }
        }
    }
}

/// Scoring identity per pass. Pass 1 hashes the short profile, pass 2 the full one.
#[derive(Debug, Clone)]
pub struct ScoringIdentities {
    pub pass1: ScoreIdentity,
    pub pass2: ScoreIdentity,
}

impl ScoringIdentities {
    pub fn for_pass(&self, pass: u8) -> &ScoreIdentity {
        if pass == 1 {
            &self.pass1
        } else {
            &self.pass2
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistOutcome {
    /// Reached the inbox as a pending row.
    Committed,
    /// Scored and recorded as a sighting, but below the cutoff — no inbox row.
    UnderCutoff,
    SuppressedByDismissal,
    /// Scored and recorded as a sighting, but it is a Job the user already tracks — no
    /// inbox row. Alert mails repeat listings for weeks; each repeat is not a new match.
    AlreadyTracked,
}

pub fn start_run(conn: &mut Connection, run_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO mail_scan_runs (run_id, status, started_at, stats_json)
         VALUES (?1, ?2, ?3, '{}')",
        params![run_id, RunStatus::Running, now],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Record which model and profiles a run used, so History can explain a score later.
pub fn record_run_identity(
    conn: &Connection,
    run_id: &str,
    model_id: &str,
    profile_short_hash: &str,
    profile_full_hash: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE mail_scan_runs
         SET model_id = ?1, profile_short_hash = ?2, profile_full_hash = ?3
         WHERE run_id = ?4",
        params![model_id, profile_short_hash, profile_full_hash, run_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_run_stats(
    conn: &mut Connection,
    run_id: &str,
    stats: &RunStats,
) -> Result<(), String> {
    let stats_json = serde_json::to_string(stats).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE mail_scan_runs SET stats_json = ?1 WHERE run_id = ?2",
        params![stats_json, run_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn finish_run(
    conn: &mut Connection,
    run_id: &str,
    status: RunStatus,
    error_code: Option<&str>,
    error_summary: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE mail_scan_runs
         SET status = ?1, finished_at = ?2, error_code = ?3, error_summary = ?4
         WHERE run_id = ?5",
        params![status, now, error_code, error_summary, run_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn upsert_source_cursor(
    conn: &mut Connection,
    source_id: &str,
    path: &str,
    kind: SourceKind,
    size: i64,
    mtime_ns: i64,
    offset: i64,
    last_message_id: Option<&str>,
    sentinel_hash: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO mail_source_cursors (
            source_id, path, kind, size, mtime_ns, offset, sentinel_hash, last_message_id, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(source_id) DO UPDATE SET
            path = excluded.path,
            kind = excluded.kind,
            size = excluded.size,
            mtime_ns = excluded.mtime_ns,
            offset = excluded.offset,
            sentinel_hash = excluded.sentinel_hash,
            last_message_id = excluded.last_message_id,
            updated_at = excluded.updated_at",
        params![
            source_id,
            path,
            kind,
            size,
            mtime_ns,
            offset,
            sentinel_hash,
            last_message_id,
            now
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// The Draft a Mail Match becomes a Job from: what the digest said, overlaid on
/// whatever the listing page added, plus the page text itself. When enrichment followed
/// the board link to the employer's ad, that ad is the `url` and the board link moves
/// to `board_url`.
///
/// Where both sources name a title or company, the extractor wins: the board's own
/// markup is more reliable than a model reading a page. A blank extractor value is not
/// a claim, though — the generic extractor has no company at all — so it only fills a
/// field enrichment left empty, and never erases one it found.
fn draft_json(listing: &ListingEvent, enrichment: &Enrichment) -> String {
    let mut draft = serde_json::Map::new();
    for (k, v) in &enrichment.partial {
        draft.insert(k.clone(), v.clone());
    }
    let mut prefer_extractor = |field: &str, value: &str| {
        if !value.trim().is_empty() || !has_text(&draft, field) {
            draft.insert(field.into(), json!(value));
        }
    };
    prefer_extractor("title", &listing.title);
    prefer_extractor("company", &listing.company);
    match enrichment.employer_url.as_deref() {
        // Followed off the board: the employer's ad is the Job's link, and the board
        // page stays reachable next to it.
        Some(ad) => {
            draft.insert("url".into(), json!(ad));
            draft.insert("board_url".into(), json!(listing.url));
        }
        None => prefer_extractor("url", &listing.url),
    }
    // The page text when it was fetched; the digest's teaser only as a fallback.
    let raw_text = enrichment.page_text.as_deref().unwrap_or(&listing.snippet);
    draft.insert("raw_text".into(), json!(raw_text));
    if let Some(board) = listing.external_ref.as_ref().map(|r| r.board.clone()) {
        draft.insert("source".into(), json!(board));
    }
    if !listing.location.trim().is_empty() && !has_text(&draft, "workplace_city") {
        draft.insert("workplace_city".into(), json!(listing.location));
    }
    Value::Object(draft).to_string()
}

fn has_text(draft: &serde_json::Map<String, Value>, field: &str) -> bool {
    draft
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
}

/// Whether a listing is already a Job on the board.
///
/// Two ways to know: this fingerprint was accepted into a Job (the link survives the
/// user editing that Job's URL, and dies with the Job through the FK cascade), or some
/// Job — accepted, captured, or typed in — carries the listing's link as its `url` or
/// its Board Link. Exact matching is deliberate: the sidecar hands over canonical
/// listing URLs, and a fuzzy company/title match would hide a second opening at the
/// same employer.
fn is_tracked(
    conn: &Connection,
    fingerprint_id: &str,
    listing: &ListingEvent,
    employer_url: Option<&str>,
) -> Result<bool, String> {
    let accepted: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM mail_match_inbox
             WHERE fingerprint_id = ?1 AND status = ?2 AND job_id IS NOT NULL
             LIMIT 1",
            params![fingerprint_id, InboxStatus::Accepted],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if accepted.is_some() {
        return Ok(true);
    }
    let links: Vec<&str> = std::iter::once(listing.url.as_str())
        .chain(employer_url)
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .collect();
    for link in links {
        let found: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM jobs WHERE url = ?1 OR board_url = ?1 LIMIT 1",
                params![link],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if found.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The gate every listing passes before it is scored or fetched (spec: "no LLM spend
/// if suppressed"). A dismissed listing, or one that is already a Job, is recorded as a
/// sighting here and never reaches the model or the network.
///
/// Returns `None` for a listing that should go on to scoring. Its cluster lookup is
/// rolled back in that case, so `persist_listing` counts the sighting exactly once.
pub fn gate_listing(
    conn: &mut Connection,
    listing: &ListingEvent,
) -> Result<Option<PersistOutcome>, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (fp_id, _) = upsert_cluster(
        &tx,
        listing.fingerprint.strong.as_deref(),
        &listing.fingerprint.weak,
        &now,
    )?;
    let outcome = if is_dismissed(&tx, &fp_id)? {
        PersistOutcome::SuppressedByDismissal
    } else if is_tracked(&tx, &fp_id, listing, None)? {
        PersistOutcome::AlreadyTracked
    } else {
        return Ok(None); // dropping `tx` rolls the cluster upsert back
    };
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Some(outcome))
}

/// One transaction per listing.
pub fn persist_listing(
    conn: &mut Connection,
    run_id: &str,
    listing: &ListingEvent,
    scored: &ScoreOutcome,
    enrichment: &Enrichment,
    identities: &ScoringIdentities,
) -> Result<PersistOutcome, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let strong = listing.fingerprint.strong.as_deref();
    let weak = listing.fingerprint.weak.as_str();

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (fp_id, near) = upsert_cluster(&tx, strong, weak, &now)?;
    let dismissed = is_dismissed(&tx, &fp_id)?;

    // A dismissal suppresses the row but never rewrites the score history — the
    // Dismissed tab has to be able to show what was suppressed and why.
    let effective_outcome = if dismissed { Verdict::UnderCutoff } else { scored.outcome };

    // Earlier passes record that the listing advanced; the last pass carries the
    // verdict, which is what `count_under_cutoff` and the re-score backlog read.
    let last = scored.passes.len().saturating_sub(1);
    for (i, pass) in scored.passes.iter().enumerate() {
        let pass_outcome = if i == last { effective_outcome } else { Verdict::Inbox };
        record_sighting(
            &tx,
            run_id,
            &fp_id,
            &scored.content_hash,
            pass.pass,
            identities.for_pass(pass.pass),
            pass.score,
            &pass.reason,
            pass_outcome,
            &now,
        )?;
    }

    if dismissed {
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(PersistOutcome::SuppressedByDismissal);
    }
    // Checked again here: enrichment may have followed the board link to an employer
    // ad that a Job already carries, which the gate could not know before the fetch.
    if is_tracked(&tx, &fp_id, listing, enrichment.employer_url.as_deref())? {
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(PersistOutcome::AlreadyTracked);
    }

    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM mail_match_inbox
             WHERE fingerprint_id = ?1 AND status = ?2",
            params![&fp_id, InboxStatus::Pending],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if effective_outcome != Verdict::Inbox {
        // Below cutoff: recorded as a sighting, no new row. An already-pending row for
        // the same listing is refreshed rather than left showing a stale score, but a
        // listing that never reached the inbox does not enter it now.
        if let Some(id) = existing {
            tx.execute(
                "UPDATE mail_match_inbox
                 SET score = ?1, score_reason = ?2, score_state = ?3,
                     suspicious = ?4, last_run_id = ?5, updated_at = ?6
                 WHERE id = ?7",
                params![
                    scored.score(),
                    scored.reason(),
                    scored.score_state(),
                    i64::from(scored.suspicious),
                    run_id,
                    &now,
                    id
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(PersistOutcome::UnderCutoff);
    }

    let draft = draft_json(listing, enrichment);
    let board = listing.external_ref.as_ref().map(|r| r.board.as_str());

    if let Some(id) = existing {
        tx.execute(
            "UPDATE mail_match_inbox SET
                score = ?1, score_reason = ?2, score_state = ?3,
                suspicious = ?4, draft_json = ?5, last_run_id = ?6, updated_at = ?7,
                listing_url = ?8, message_id = ?9, message_date = ?10,
                source_board = ?11, enrichment_state = ?12, enrichment_error = ?13,
                near_duplicate_of = ?14
             WHERE id = ?15",
            params![
                scored.score(),
                scored.reason(),
                scored.score_state(),
                i64::from(scored.suspicious),
                &draft,
                run_id,
                &now,
                &listing.url,
                &listing.message_id,
                &listing.message_date,
                board,
                enrichment.state,
                enrichment.error.as_deref(),
                near.as_deref(),
                id
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "INSERT INTO mail_match_inbox (
                fingerprint_id, kind, status, score, score_reason, score_state,
                suspicious, near_duplicate_of, draft_json, enrichment_state,
                enrichment_error, source_board, message_id, message_date, listing_url,
                first_run_id, last_run_id, created_at, updated_at
             ) VALUES (
                ?1, 'new', ?16, ?2, ?3, ?4,
                ?5, ?6, ?7, ?8,
                ?9, ?10, ?11, ?12, ?13,
                ?14, ?14, ?15, ?15
             )",
            params![
                &fp_id,
                scored.score(),
                scored.reason(),
                scored.score_state(),
                i64::from(scored.suspicious),
                near.as_deref(),
                &draft,
                enrichment.state,
                enrichment.error.as_deref(),
                board,
                &listing.message_id,
                &listing.message_date,
                &listing.url,
                run_id,
                &now,
                InboxStatus::Pending
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(PersistOutcome::Committed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail_scan::cluster::{dismiss, restore};
    use crate::mail_scan::protocol::{FingerprintKeys, ListingEvent};
    use crate::mail_scan::scoring::{PassRecord, ScoreOutcome, ScoringEngine};
    use crate::migrations;

    pub(crate) fn listing(title: &str, strong: &str, weak: &str) -> ListingEvent {
        ListingEvent {
            source: "indeed".into(),
            message_id: "<m1@x>".into(),
            message_date: "2026-09-08T06:12:00Z".into(),
            seq: 0,
            title: title.into(),
            company: "Acme".into(),
            location: "Kbh".into(),
            url: format!("https://dk.indeed.com/viewjob?jk={strong}"),
            external_ref: None,
            snippet: "snippet".into(),
            posted_at: None,
            fingerprint: FingerprintKeys {
                strong: Some(strong.into()),
                weak: weak.into(),
            },
            extractor: "indeed".into(),
            extractor_confidence: 0.9,
        }
    }

    fn identities() -> ScoringIdentities {
        let engine = ScoringEngine::stub();
        ScoringIdentities {
            pass1: engine.identity(1),
            pass2: engine.identity(2),
        }
    }

    fn outcome(score: i32, verdict: Verdict) -> ScoreOutcome {
        ScoreOutcome {
            content_hash: format!("hash-{score}-{verdict}"),
            passes: vec![PassRecord {
                pass: 1,
                score: Some(score),
                reason: "because".into(),
                cached: false,
            }],
            suspicious: false,
            outcome: verdict,
        }
    }

    fn scored_for(listing: &ListingEvent, verdict: Verdict) -> ScoreOutcome {
        ScoreOutcome {
            content_hash: crate::mail_scan::scoring::listing_content_hash(listing),
            passes: vec![PassRecord {
                pass: 1,
                score: Some(8),
                reason: "because".into(),
                cached: false,
            }],
            suspicious: false,
            outcome: verdict,
        }
    }

    pub(crate) fn db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();
        conn
    }

    #[test]
    fn crash_mid_stream_keeps_committed_items() {
        let mut conn = db();
        let stream = r#"
{"t":"started","protocol":2,"run_id":"r1","sidecar_version":"1.0.0","sources":1}
{"t":"listing","source":"indeed","message_id":"<a>","message_date":"2026-09-08T06:12:00Z","seq":0,"title":"A","company":"Acme","location":"Kbh","url":"https://example.com/a","snippet":"s","fingerprint":{"strong":"indeed:a","weak":"acme|a|kbh"},"extractor":"indeed","extractor_confidence":0.9}
{"t":"listing","source":"indeed","message_id":"<b>","message_date":"2026-09-08T06:12:00Z","seq":1,"title":"B","company":"Acme","location":"Kbh","url":"https://example.com/b","snippet":"s","fingerprint":{"strong":"indeed:b","weak":"acme|b|kbh"},"extractor":"indeed","extractor_confidence":0.9}
{"t":"listing","source":"indeed","message_id":"<c","broken
"#;
        let mut engine = ScoringEngine::stub();
        let stats =
            crate::mail_scan::consume_event_stream(&mut conn, "r1", stream.as_bytes(), &mut engine)
                .unwrap();
        assert_eq!(stats.listings_committed, 2);
        let status: String = conn
            .query_row(
                "SELECT status FROM mail_scan_runs WHERE run_id = 'r1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "failed");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM mail_match_inbox", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn persist_listing_upserts_fingerprint() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:x", "acme|dev|kbh");
        let scored = scored_for(&l, Verdict::Inbox);
        persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities()).unwrap();
        persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities()).unwrap();

        let seen: i64 = conn
            .query_row(
                "SELECT seen_count FROM mail_fingerprints WHERE fingerprint_id = 'indeed:x'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(seen, 2);
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_match_inbox WHERE status = 'pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending, 1);
    }

    #[test]
    fn dismiss_suppresses_rescan_restore_readmits() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let a = listing("Dev", "indeed:keep", "acme|dev|kbh");
        let b = listing("Other", "indeed:other", "other|role|kbh");
        let sa = scored_for(&a, Verdict::Inbox);
        let sb = scored_for(&b, Verdict::Inbox);
        let ids = identities();

        assert_eq!(
            persist_listing(&mut conn, "r1", &a, &sa, &Enrichment::skipped(), &ids).unwrap(),
            PersistOutcome::Committed
        );
        assert_eq!(
            persist_listing(&mut conn, "r1", &b, &sb, &Enrichment::skipped(), &ids).unwrap(),
            PersistOutcome::Committed
        );

        dismiss(&mut conn, "indeed:keep", Some("r1"), Some("noise")).unwrap();

        start_run(&mut conn, "r2").unwrap();
        assert_eq!(
            persist_listing(&mut conn, "r2", &a, &sa, &Enrichment::skipped(), &ids).unwrap(),
            PersistOutcome::SuppressedByDismissal
        );
        // Different listing must not be buried by the dismissal.
        assert_eq!(
            persist_listing(&mut conn, "r2", &b, &sb, &Enrichment::skipped(), &ids).unwrap(),
            PersistOutcome::Committed
        );

        let pending_keep: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_match_inbox
                 WHERE fingerprint_id = 'indeed:keep' AND status = 'pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending_keep, 0);

        restore(&mut conn, "indeed:keep").unwrap();
        start_run(&mut conn, "r3").unwrap();
        assert_eq!(
            persist_listing(&mut conn, "r3", &a, &sa, &Enrichment::skipped(), &ids).unwrap(),
            PersistOutcome::Committed
        );
        let pending_keep: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_match_inbox
                 WHERE fingerprint_id = 'indeed:keep' AND status = 'pending'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pending_keep, 1);
    }

    fn track_job(conn: &Connection, url: Option<&str>, board_url: Option<&str>) -> i64 {
        conn.execute(
            "INSERT INTO jobs (company, title, status, url, board_url, created_at, updated_at)
             VALUES ('Acme', 'Dev', 'Application Sent', ?1, ?2, 't', 't')",
            params![url, board_url],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    fn pending_count(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM mail_match_inbox WHERE status = 'pending'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn seen_count(conn: &Connection, fingerprint_id: &str) -> i64 {
        conn.query_row(
            "SELECT seen_count FROM mail_fingerprints WHERE fingerprint_id = ?1",
            params![fingerprint_id],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn a_listing_the_user_already_tracks_is_seen_but_not_queued() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:tracked", "acme|dev|kbh");
        track_job(&conn, Some(&l.url), None);
        let scored = scored_for(&l, Verdict::Inbox);

        let outcome =
            persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities())
                .unwrap();

        assert_eq!(outcome, PersistOutcome::AlreadyTracked);
        assert_eq!(pending_count(&conn), 0, "a tracked job must not come back as a match");
        assert_eq!(seen_count(&conn, "indeed:tracked"), 1, "but the sighting is recorded");
        let sightings: i64 = conn
            .query_row("SELECT COUNT(*) FROM mail_scored_sightings", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sightings, 1, "and its score is cached, so a re-run is free");
    }

    #[test]
    fn a_listing_whose_board_link_a_job_keeps_is_not_queued() {
        // The Job's url is the employer's ad; the board page it was found through is
        // its Board Link — and that is what the next alert mail links to.
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "jobindex:h7", "acme|dev|kbh");
        track_job(&conn, Some("https://careers.acme.example/ad/7"), Some(&l.url));
        let scored = scored_for(&l, Verdict::Inbox);

        let outcome =
            persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities())
                .unwrap();

        assert_eq!(outcome, PersistOutcome::AlreadyTracked);
        assert_eq!(pending_count(&conn), 0);
    }

    #[test]
    fn an_accepted_match_seen_again_does_not_return_to_the_inbox() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:accepted", "acme|dev|kbh");
        let scored = scored_for(&l, Verdict::Inbox);
        persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities())
            .unwrap();
        let inbox_id: i64 = conn
            .query_row("SELECT id FROM mail_match_inbox", [], |r| r.get(0))
            .unwrap();
        crate::mail_scan::accept::accept_new(&mut conn, inbox_id).unwrap();
        // The user edits the Job's link afterwards; the match still knows its Job.
        conn.execute("UPDATE jobs SET url = 'https://careers.acme.example/x'", [])
            .unwrap();

        start_run(&mut conn, "r2").unwrap();
        let outcome =
            persist_listing(&mut conn, "r2", &l, &scored, &Enrichment::skipped(), &identities())
                .unwrap();

        assert_eq!(outcome, PersistOutcome::AlreadyTracked);
        assert_eq!(pending_count(&conn), 0);
        assert_eq!(seen_count(&conn, "indeed:accepted"), 2);
    }

    #[test]
    fn a_different_listing_at_the_same_company_still_reaches_the_inbox() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        track_job(&conn, Some("https://dk.indeed.com/viewjob?jk=indeed:other"), None);
        let l = listing("Dev", "indeed:new-one", "acme|dev|kbh");
        let scored = scored_for(&l, Verdict::Inbox);

        let outcome =
            persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities())
                .unwrap();

        assert_eq!(outcome, PersistOutcome::Committed);
        assert_eq!(pending_count(&conn), 1);
    }

    #[test]
    fn under_cutoff_records_a_sighting_but_no_inbox_row() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:low", "acme|dev|kbh");
        let mut scored = outcome(3, Verdict::UnderCutoff);
        scored.content_hash = crate::mail_scan::scoring::listing_content_hash(&l);

        assert_eq!(
            persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities()).unwrap(),
            PersistOutcome::UnderCutoff
        );

        let inbox: i64 = conn
            .query_row("SELECT COUNT(*) FROM mail_match_inbox", [], |r| r.get(0))
            .unwrap();
        assert_eq!(inbox, 0, "an under-cutoff listing must not reach the inbox");

        let sightings: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_scored_sightings WHERE outcome = 'under_cutoff'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sightings, 1, "but it must be remembered, so it is not re-paid for");
    }

    #[test]
    fn both_passes_are_recorded_with_their_own_profile_hash() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:two", "acme|dev|kbh");
        let scored = ScoreOutcome {
            content_hash: crate::mail_scan::scoring::listing_content_hash(&l),
            passes: vec![
                PassRecord { pass: 1, score: Some(8), reason: "gate".into(), cached: false },
                PassRecord { pass: 2, score: Some(9), reason: "deep".into(), cached: false },
            ],
            suspicious: false,
            outcome: Verdict::Inbox,
        };
        let ids = ScoringIdentities {
            pass1: ScoreIdentity {
                profile_hash: "short-hash".into(),
                prompt_version: "v1".into(),
                model_id: "m1".into(),
            },
            pass2: ScoreIdentity {
                profile_hash: "full-hash".into(),
                prompt_version: "v1".into(),
                model_id: "m1".into(),
            },
        };

        persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &ids).unwrap();

        let mut stmt = conn
            .prepare("SELECT pass, profile_hash, score FROM mail_scored_sightings ORDER BY pass")
            .unwrap();
        let rows: Vec<(i64, String, i64)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(
            rows,
            vec![
                (1, "short-hash".to_string(), 8),
                (2, "full-hash".to_string(), 9)
            ]
        );

        // The inbox shows the final (pass-2) verdict, not the gate score.
        let score: i64 = conn
            .query_row("SELECT score FROM mail_match_inbox", [], |r| r.get(0))
            .unwrap();
        assert_eq!(score, 9);
    }

    /// Enrichment failure must never cost the user a listing (spec §5.2).
    ///
    /// A dead link, a timeout, and an over-size body are all the same story from the
    /// inbox's point of view: the match is still worth reviewing, and the row has to
    /// say what went wrong so "incomplete enrichment is visible, not silent".
    #[test]
    fn a_failed_enrichment_still_enqueues_the_match_with_a_reason() {
        for reason in [
            "could not fetch the listing: Request failed",
            "connection timed out",
            "Response body exceeds 2 MiB",
        ] {
            let mut conn = db();
            start_run(&mut conn, "r1").unwrap();
            let l = listing("Dev", &format!("indeed:{}", reason.len()), "acme|dev|kbh");
            let scored = scored_for(&l, Verdict::Inbox);

            let outcome = persist_listing(
                &mut conn,
                "r1",
                &l,
                &scored,
                &Enrichment::failed(reason),
                &identities(),
            )
            .unwrap();

            assert_eq!(outcome, PersistOutcome::Committed, "{reason}");
            let (state, error): (String, Option<String>) = conn
                .query_row(
                    "SELECT enrichment_state, enrichment_error FROM mail_match_inbox",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert_eq!(state, "failed", "{reason}");
            assert!(error.is_some(), "the row must say why: {reason}");
        }
    }

    #[test]
    fn enrichment_fills_the_draft_without_overruling_the_extractor() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:enriched", "acme|dev|kbh");
        let scored = scored_for(&l, Verdict::Inbox);

        let mut partial = std::collections::HashMap::new();
        partial.insert("deadline".to_string(), json!("2026-10-01"));
        partial.insert("salary_range".to_string(), json!("60-70k DKK"));
        // A model reading the page thinks the title is something else. The board's own
        // markup is more reliable, so the extractor wins.
        partial.insert("title".to_string(), json!("Totally Different Title"));
        let enrichment = Enrichment::from_partial(partial);

        persist_listing(&mut conn, "r1", &l, &scored, &enrichment, &identities()).unwrap();

        let draft: String = conn
            .query_row("SELECT draft_json FROM mail_match_inbox", [], |r| r.get(0))
            .unwrap();
        let v: Value = serde_json::from_str(&draft).unwrap();
        assert_eq!(v["deadline"], json!("2026-10-01"));
        assert_eq!(v["salary_range"], json!("60-70k DKK"));
        assert_eq!(v["title"], json!("Dev"), "the extractor's title must win");
        assert_eq!(v["company"], json!("Acme"));
    }

    fn stored_draft(conn: &Connection) -> Value {
        let draft: String = conn
            .query_row("SELECT draft_json FROM mail_match_inbox", [], |r| r.get(0))
            .unwrap();
        serde_json::from_str(&draft).unwrap()
    }

    #[test]
    fn the_draft_keeps_the_fetched_listing_text_rather_than_the_digest_teaser() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:page", "acme|dev|kbh");
        let scored = scored_for(&l, Verdict::Inbox);
        let page = "Rust Developer at Acme. You will build the ingestion pipeline. \
                    Apply by 1 October.";
        let enrichment = Enrichment::from_partial(std::collections::HashMap::new())
            .with_page_text(page);

        persist_listing(&mut conn, "r1", &l, &scored, &enrichment, &identities()).unwrap();

        assert_eq!(stored_draft(&conn)["raw_text"], json!(page));
    }

    #[test]
    fn a_followed_listing_drafts_the_employer_ad_and_keeps_the_board_link() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let mut l = listing("Dev", "jobindex:h1", "acme|dev|kbh");
        l.url = "https://www.jobindex.dk/c?t=h1".into();
        let scored = scored_for(&l, Verdict::Inbox);
        let enrichment = Enrichment {
            employer_url: Some("https://candidate.hr-manager.net/ad/1".into()),
            ..Enrichment::from_partial(std::collections::HashMap::new()).with_page_text("ad")
        };

        persist_listing(&mut conn, "r1", &l, &scored, &enrichment, &identities()).unwrap();

        let draft = stored_draft(&conn);
        assert_eq!(draft["url"], json!("https://candidate.hr-manager.net/ad/1"));
        assert_eq!(draft["board_url"], json!("https://www.jobindex.dk/c?t=h1"));
    }

    #[test]
    fn an_unfollowed_listing_has_no_board_link() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "linkedin:1", "acme|dev|kbh");
        let scored = scored_for(&l, Verdict::Inbox);

        persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities())
            .unwrap();

        let draft = stored_draft(&conn);
        assert_eq!(draft["url"], json!(l.url));
        assert!(draft.get("board_url").is_none(), "{draft}");
    }

    #[test]
    fn without_a_fetched_page_the_draft_falls_back_to_the_digest_snippet() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:nopage", "acme|dev|kbh");
        let scored = scored_for(&l, Verdict::Inbox);

        persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::failed("timeout"), &identities())
            .unwrap();

        assert_eq!(stored_draft(&conn)["raw_text"], json!("snippet"));
    }

    #[test]
    fn a_blank_extractor_value_never_erases_what_enrichment_found() {
        // The generic extractor knows only the subject line: it has no company, and
        // sometimes no location. The page does.
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let mut l = listing("Dev", "indeed:generic", "acme|dev|kbh");
        l.company = String::new();
        l.location = "  ".into();
        let scored = scored_for(&l, Verdict::Inbox);

        let mut partial = std::collections::HashMap::new();
        partial.insert("company".to_string(), json!("Acme A/S"));
        partial.insert("workplace_city".to_string(), json!("Aarhus"));
        let enrichment = Enrichment::from_partial(partial);

        persist_listing(&mut conn, "r1", &l, &scored, &enrichment, &identities()).unwrap();

        let draft = stored_draft(&conn);
        assert_eq!(draft["company"], json!("Acme A/S"));
        assert_eq!(draft["workplace_city"], json!("Aarhus"));
        assert_eq!(draft["title"], json!("Dev"), "a non-blank extractor title still wins");
    }

    #[test]
    fn invalid_score_lands_in_the_inbox_with_a_null_score() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:bad", "acme|dev|kbh");
        let scored = ScoreOutcome {
            content_hash: crate::mail_scan::scoring::listing_content_hash(&l),
            passes: vec![PassRecord {
                pass: 1,
                score: None,
                reason: "model returned out-of-range score 42".into(),
                cached: false,
            }],
            suspicious: true,
            outcome: Verdict::Inbox,
        };

        persist_listing(&mut conn, "r1", &l, &scored, &Enrichment::skipped(), &identities()).unwrap();

        let (score, state, suspicious): (Option<i64>, String, i64) = conn
            .query_row(
                "SELECT score, score_state, suspicious FROM mail_match_inbox",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(score, None, "an invalid answer must never become a number");
        assert_eq!(state, "invalid");
        assert_eq!(suspicious, 1);
    }
}
