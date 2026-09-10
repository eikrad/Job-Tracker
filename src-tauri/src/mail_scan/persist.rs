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
    pub updates: u32,
    pub llm_calls: u32,
    pub enrichment_failures: u32,
    pub errors: u32,
    /// Run hit the call cap. Not a failure — the run still `completed`.
    pub budget_exhausted: bool,
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
}

pub fn start_run(conn: &mut Connection, run_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO mail_scan_runs (run_id, status, started_at, stats_json)
         VALUES (?1, 'running', ?2, '{}')",
        params![run_id, now],
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
    status: &str,
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
    kind: &str,
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

/// The prefill for the job form: what the digest said, overlaid with whatever the
/// listing page added.
///
/// Enrichment fills gaps, it does not overrule the extractor: title and company come
/// from the board's own markup, which is more reliable than a model reading a page.
fn draft_json(listing: &ListingEvent, enrichment: &Enrichment) -> String {
    let mut draft = serde_json::Map::new();
    for (k, v) in &enrichment.partial {
        draft.insert(k.clone(), v.clone());
    }
    draft.insert("title".into(), json!(listing.title));
    draft.insert("company".into(), json!(listing.company));
    draft.insert("url".into(), json!(listing.url));
    draft.insert("raw_text".into(), json!(listing.snippet));
    if let Some(board) = listing.external_ref.as_ref().map(|r| r.board.clone()) {
        draft.insert("source".into(), json!(board));
    }
    if !listing.location.trim().is_empty() && !draft.contains_key("workplace_city") {
        draft.insert("workplace_city".into(), json!(listing.location));
    }
    Value::Object(draft).to_string()
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
    let effective_outcome = if dismissed { "under_cutoff" } else { scored.outcome };

    // Earlier passes record that the listing advanced; the last pass carries the
    // verdict, which is what `count_under_cutoff` and the re-score backlog read.
    let last = scored.passes.len().saturating_sub(1);
    for (i, pass) in scored.passes.iter().enumerate() {
        let pass_outcome = if i == last { effective_outcome } else { "inbox" };
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

    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM mail_match_inbox
             WHERE fingerprint_id = ?1 AND kind = 'new' AND status = 'pending'",
            params![&fp_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if effective_outcome != "inbox" {
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
                ?1, 'new', 'pending', ?2, ?3, ?4,
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
                &now
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

    fn outcome(score: i32, verdict: &'static str) -> ScoreOutcome {
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

    fn scored_for(listing: &ListingEvent, verdict: &'static str) -> ScoreOutcome {
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
{"t":"started","protocol":1,"run_id":"r1","sidecar_version":"1.0.0","sources":1}
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
        let scored = scored_for(&l, "inbox");
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
        let sa = scored_for(&a, "inbox");
        let sb = scored_for(&b, "inbox");
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

    #[test]
    fn under_cutoff_records_a_sighting_but_no_inbox_row() {
        let mut conn = db();
        start_run(&mut conn, "r1").unwrap();
        let l = listing("Dev", "indeed:low", "acme|dev|kbh");
        let mut scored = outcome(3, "under_cutoff");
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
            outcome: "inbox",
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
            let scored = scored_for(&l, "inbox");

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
        let scored = scored_for(&l, "inbox");

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
            outcome: "inbox",
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
