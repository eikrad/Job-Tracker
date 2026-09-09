//! Per-item persistence for mail scan listings (spec §7.3).

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::json;

use crate::mail_scan::cluster::{is_dismissed, upsert_cluster};
use crate::mail_scan::protocol::ListingEvent;
use crate::mail_scan::scoring::ListingScorer;

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStats {
    pub listings_committed: u32,
    pub messages_seen: u32,
    pub messages_parsed: u32,
    pub suppressed_by_dismissal: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistOutcome {
    Committed,
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

/// One transaction per listing.
pub fn persist_listing(
    conn: &mut Connection,
    run_id: &str,
    listing: &ListingEvent,
    scorer: &dyn ListingScorer,
) -> Result<PersistOutcome, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let score = scorer.score(listing);
    let strong = listing.fingerprint.strong.as_deref();
    let weak = listing.fingerprint.weak.as_str();

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (fp_id, near) = upsert_cluster(&tx, strong, weak, &now)?;

    if is_dismissed(&tx, &fp_id)? {
        tx.execute(
            "INSERT INTO mail_scored_sightings (
                fingerprint_id, pass, score, reason, profile_hash, prompt_version,
                model_id, listing_content_hash, outcome, scored_at, run_id
             ) VALUES (?1, 1, ?2, ?3, 'stub', 'stub-v0', 'stub', ?4, 'under_cutoff', ?5, ?6)
             ON CONFLICT(listing_content_hash, pass, profile_hash, prompt_version, model_id)
             DO NOTHING",
            params![
                &fp_id,
                score.score,
                &score.reason,
                content_hash(listing),
                &now,
                run_id
            ],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(PersistOutcome::SuppressedByDismissal);
    }

    tx.execute(
        "INSERT INTO mail_scored_sightings (
            fingerprint_id, pass, score, reason, profile_hash, prompt_version,
            model_id, listing_content_hash, outcome, scored_at, run_id
         ) VALUES (?1, 1, ?2, ?3, 'stub', 'stub-v0', 'stub', ?4, ?5, ?6, ?7)
         ON CONFLICT(listing_content_hash, pass, profile_hash, prompt_version, model_id)
         DO NOTHING",
        params![
            &fp_id,
            score.score,
            &score.reason,
            content_hash(listing),
            score.outcome,
            &now,
            run_id
        ],
    )
    .map_err(|e| e.to_string())?;

    let draft = json!({
        "title": listing.title,
        "company": listing.company,
        "url": listing.url,
        "raw_text": listing.snippet,
        "source": listing.external_ref.as_ref().map(|r| r.board.clone()),
    })
    .to_string();

    let existing: Option<i64> = tx
        .query_row(
            "SELECT id FROM mail_match_inbox
             WHERE fingerprint_id = ?1 AND kind = 'new' AND status = 'pending'",
            params![&fp_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    if let Some(id) = existing {
        tx.execute(
            "UPDATE mail_match_inbox SET
                score = ?1, score_reason = ?2, score_state = ?3,
                draft_json = ?4, last_run_id = ?5, updated_at = ?6,
                listing_url = ?7, message_id = ?8, message_date = ?9,
                source_board = ?10, enrichment_state = ?11,
                near_duplicate_of = ?12
             WHERE id = ?13",
            params![
                score.score,
                &score.reason,
                score.score_state,
                &draft,
                run_id,
                &now,
                &listing.url,
                &listing.message_id,
                &listing.message_date,
                listing.external_ref.as_ref().map(|r| r.board.as_str()),
                score.enrichment_state,
                near.as_deref(),
                id
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "INSERT INTO mail_match_inbox (
                fingerprint_id, kind, status, score, score_reason, score_state,
                near_duplicate_of, draft_json, enrichment_state, source_board, message_id,
                message_date, listing_url, first_run_id, last_run_id, created_at, updated_at
             ) VALUES (
                ?1, 'new', 'pending', ?2, ?3, ?4,
                ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?12, ?13, ?13
             )",
            params![
                &fp_id,
                score.score,
                &score.reason,
                score.score_state,
                near.as_deref(),
                &draft,
                score.enrichment_state,
                listing.external_ref.as_ref().map(|r| r.board.as_str()),
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

fn content_hash(listing: &ListingEvent) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(listing.url.as_bytes());
    hasher.update(b"|");
    hasher.update(listing.title.as_bytes());
    hasher.update(b"|");
    hasher.update(listing.company.as_bytes());
    hasher.update(b"|");
    hasher.update(listing.snippet.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mail_scan::cluster::{dismiss, restore};
    use crate::mail_scan::protocol::{FingerprintKeys, ListingEvent};
    use crate::mail_scan::scoring::StubScorer;
    use crate::migrations;

    fn listing(title: &str, strong: &str, weak: &str) -> ListingEvent {
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

    #[test]
    fn crash_mid_stream_keeps_committed_items() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();

        let stream = r#"
{"t":"started","protocol":1,"run_id":"r1","sidecar_version":"1.0.0","sources":1}
{"t":"listing","source":"indeed","message_id":"<a>","message_date":"2026-09-08T06:12:00Z","seq":0,"title":"A","company":"Acme","location":"Kbh","url":"https://example.com/a","snippet":"s","fingerprint":{"strong":"indeed:a","weak":"acme|a|kbh"},"extractor":"indeed","extractor_confidence":0.9}
{"t":"listing","source":"indeed","message_id":"<b>","message_date":"2026-09-08T06:12:00Z","seq":1,"title":"B","company":"Acme","location":"Kbh","url":"https://example.com/b","snippet":"s","fingerprint":{"strong":"indeed:b","weak":"acme|b|kbh"},"extractor":"indeed","extractor_confidence":0.9}
{"t":"listing","source":"indeed","message_id":"<c","broken
"#;
        let stats =
            crate::mail_scan::consume_event_stream(&mut conn, "r1", stream.as_bytes(), &StubScorer)
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
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();
        start_run(&mut conn, "r1").unwrap();
        persist_listing(
            &mut conn,
            "r1",
            &listing("Dev", "indeed:x", "acme|dev|kbh"),
            &StubScorer,
        )
        .unwrap();
        persist_listing(
            &mut conn,
            "r1",
            &listing("Dev", "indeed:x", "acme|dev|kbh"),
            &StubScorer,
        )
        .unwrap();
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
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();
        start_run(&mut conn, "r1").unwrap();
        let a = listing("Dev", "indeed:keep", "acme|dev|kbh");
        let b = listing("Other", "indeed:other", "other|role|kbh");
        assert_eq!(
            persist_listing(&mut conn, "r1", &a, &StubScorer).unwrap(),
            PersistOutcome::Committed
        );
        assert_eq!(
            persist_listing(&mut conn, "r1", &b, &StubScorer).unwrap(),
            PersistOutcome::Committed
        );

        dismiss(&mut conn, "indeed:keep", Some("r1"), Some("noise")).unwrap();

        start_run(&mut conn, "r2").unwrap();
        assert_eq!(
            persist_listing(&mut conn, "r2", &a, &StubScorer).unwrap(),
            PersistOutcome::SuppressedByDismissal
        );
        // Different listing must not be buried by the dismissal.
        assert_eq!(
            persist_listing(&mut conn, "r2", &b, &StubScorer).unwrap(),
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
            persist_listing(&mut conn, "r3", &a, &StubScorer).unwrap(),
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
}
