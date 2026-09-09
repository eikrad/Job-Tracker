//! Per-item persistence for mail scan listings (spec §7.3).

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;

use crate::mail_scan::protocol::ListingEvent;
use crate::mail_scan::scoring::ListingScorer;

#[derive(Debug, Default, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStats {
    pub listings_committed: u32,
    pub messages_seen: u32,
    pub suppressed_by_dismissal: u32,
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

pub fn update_run_stats(conn: &mut Connection, run_id: &str, stats: &RunStats) -> Result<(), String> {
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

fn fingerprint_id(listing: &ListingEvent) -> String {
    if let Some(strong) = listing.fingerprint.strong.as_ref().filter(|s| !s.is_empty()) {
        strong.clone()
    } else {
        format!("weak:{}", listing.fingerprint.weak)
    }
}

/// One transaction per listing.
pub fn persist_listing(
    conn: &mut Connection,
    run_id: &str,
    listing: &ListingEvent,
    scorer: &dyn ListingScorer,
) -> Result<(), String> {
    let fp_id = fingerprint_id(listing);
    let now = chrono::Utc::now().to_rfc3339();
    let score = scorer.score(listing);

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let dismissed: Option<String> = tx
        .query_row(
            "SELECT fingerprint_id FROM mail_match_dismissals WHERE fingerprint_id = ?1",
            params![&fp_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO mail_fingerprints (
            fingerprint_id, strong_key, weak_key, first_seen_at, last_seen_at, seen_count
         ) VALUES (?1, ?2, ?3, ?4, ?4, 1)
         ON CONFLICT(fingerprint_id) DO UPDATE SET
            last_seen_at = excluded.last_seen_at,
            seen_count = seen_count + 1,
            strong_key = COALESCE(excluded.strong_key, mail_fingerprints.strong_key)",
        params![
            &fp_id,
            listing.fingerprint.strong.as_deref(),
            &listing.fingerprint.weak,
            &now
        ],
    )
    .map_err(|e| e.to_string())?;

    if dismissed.is_some() {
        // Still record a sighting so rescans stay accountable; inbox stays empty.
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
        return Ok(());
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
                source_board = ?10, enrichment_state = ?11
             WHERE id = ?12",
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
                listing
                    .external_ref
                    .as_ref()
                    .map(|r| r.board.as_str()),
                score.enrichment_state,
                id
            ],
        )
        .map_err(|e| e.to_string())?;
    } else {
        tx.execute(
            "INSERT INTO mail_match_inbox (
                fingerprint_id, kind, status, score, score_reason, score_state,
                draft_json, enrichment_state, source_board, message_id, message_date,
                listing_url, first_run_id, last_run_id, created_at, updated_at
             ) VALUES (
                ?1, 'new', 'pending', ?2, ?3, ?4,
                ?5, ?6, ?7, ?8, ?9,
                ?10, ?11, ?11, ?12, ?12
             )",
            params![
                &fp_id,
                score.score,
                &score.reason,
                score.score_state,
                &draft,
                score.enrichment_state,
                listing
                    .external_ref
                    .as_ref()
                    .map(|r| r.board.as_str()),
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
    Ok(())
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
    use crate::mail_scan::protocol::{FingerprintKeys, ListingEvent};
    use crate::mail_scan::scoring::StubScorer;
    use crate::migrations;

    fn listing(title: &str, strong: &str) -> ListingEvent {
        ListingEvent {
            source: "indeed".into(),
            message_id: "<m1@x>".into(),
            message_date: "2026-09-08T06:12:00Z".into(),
            seq: 0,
            title: title.into(),
            company: "Acme".into(),
            location: "Kbh".into(),
            url: "https://dk.indeed.com/viewjob?jk=abc".into(),
            external_ref: None,
            snippet: "snippet".into(),
            posted_at: None,
            fingerprint: FingerprintKeys {
                strong: Some(strong.into()),
                weak: "acme|dev|kbh".into(),
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
        let stats = crate::mail_scan::consume_event_stream(
            &mut conn,
            "r1",
            stream.as_bytes(),
            &StubScorer,
        )
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
        persist_listing(&mut conn, "r1", &listing("Dev", "indeed:x"), &StubScorer).unwrap();
        persist_listing(&mut conn, "r1", &listing("Dev", "indeed:x"), &StubScorer).unwrap();
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
}
