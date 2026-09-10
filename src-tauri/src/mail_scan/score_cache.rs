//! Scored-sighting cache and the re-score policy (spec §5.4).
//!
//! Two lookups, and the difference between them is the whole point:
//!
//! - [`lookup_exact`] is the **cost cache**, keyed by the full 5-tuple including
//!   `model_id`. A crashed run re-reading the same mail is a hit and issues no HTTP.
//! - [`lookup_reusable`] is the **re-score policy**, keyed by content + profile +
//!   prompt and deliberately blind to `model_id`. Switching provider must not
//!   re-spend the entire backlog; only a profile edit or a prompt change may.
//!
//! `profile_hash` is a hash of file *contents* ([`super::profiles`]), so a copy or a
//! cloud-sync round-trip — which moves mtime but not bytes — is a hit.

use rusqlite::{params, Connection, OptionalExtension};

/// What identifies one scoring call. Pass 1 carries the short profile's hash, pass 2
/// the full profile's, so editing only the long CV re-scores only the deep pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreIdentity {
    pub profile_hash: String,
    pub prompt_version: String,
    pub model_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedScore {
    /// `None` means the model's answer failed validation — recorded so we do not pay
    /// to be told the same nonsense twice.
    pub score: Option<i32>,
    pub reason: String,
    pub outcome: String,
}

fn row_to_cached(row: &rusqlite::Row<'_>) -> rusqlite::Result<CachedScore> {
    Ok(CachedScore {
        score: row.get(0)?,
        reason: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
        outcome: row.get(2)?,
    })
}

/// Exact-key cache hit. Hits [`idx_sighting_cache`] directly.
pub fn lookup_exact(
    conn: &Connection,
    listing_content_hash: &str,
    pass: u8,
    id: &ScoreIdentity,
) -> Result<Option<CachedScore>, String> {
    conn.query_row(
        "SELECT score, reason, outcome FROM mail_scored_sightings
         WHERE listing_content_hash = ?1 AND pass = ?2
           AND profile_hash = ?3 AND prompt_version = ?4 AND model_id = ?5",
        params![
            listing_content_hash,
            pass as i64,
            &id.profile_hash,
            &id.prompt_version,
            &id.model_id
        ],
        row_to_cached,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Model-blind reuse. The most recent sighting wins when several models have scored
/// the same listing against the same profile and prompt.
pub fn lookup_reusable(
    conn: &Connection,
    listing_content_hash: &str,
    pass: u8,
    profile_hash: &str,
    prompt_version: &str,
) -> Result<Option<CachedScore>, String> {
    conn.query_row(
        "SELECT score, reason, outcome FROM mail_scored_sightings
         WHERE listing_content_hash = ?1 AND pass = ?2
           AND profile_hash = ?3 AND prompt_version = ?4
         ORDER BY scored_at DESC LIMIT 1",
        params![
            listing_content_hash,
            pass as i64,
            profile_hash,
            prompt_version
        ],
        row_to_cached,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Insert a sighting inside the caller's transaction. The unique index makes a repeat
/// insert a no-op, so replaying a run cannot double-count.
#[allow(clippy::too_many_arguments)]
pub fn record_sighting(
    tx: &Connection,
    run_id: &str,
    fingerprint_id: &str,
    listing_content_hash: &str,
    pass: u8,
    id: &ScoreIdentity,
    score: Option<i32>,
    reason: &str,
    outcome: &str,
    now: &str,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO mail_scored_sightings (
            fingerprint_id, pass, score, reason, profile_hash, prompt_version,
            model_id, listing_content_hash, outcome, scored_at, run_id
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(listing_content_hash, pass, profile_hash, prompt_version, model_id)
         DO UPDATE SET score = excluded.score, reason = excluded.reason,
                       outcome = excluded.outcome, scored_at = excluded.scored_at,
                       run_id = excluded.run_id",
        params![
            fingerprint_id,
            pass as i64,
            score,
            reason,
            &id.profile_hash,
            &id.prompt_version,
            &id.model_id,
            listing_content_hash,
            outcome,
            now,
            run_id
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// How many listings the explicit "re-score backlog" action would pay for, so the
/// count can be shown before the user commits to spending it.
pub fn count_under_cutoff(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT COUNT(DISTINCT fingerprint_id) FROM mail_scored_sightings
         WHERE outcome = 'under_cutoff'",
        [],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
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
            "INSERT INTO mail_scan_runs (run_id, status, started_at) VALUES ('r1','running','t')",
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

    fn identity(profile: &str, prompt: &str, model: &str) -> ScoreIdentity {
        ScoreIdentity {
            profile_hash: profile.into(),
            prompt_version: prompt.into(),
            model_id: model.into(),
        }
    }

    fn seed(conn: &Connection, id: &ScoreIdentity, score: i32, outcome: &str) {
        record_sighting(
            conn, "r1", "fp1", "hash-a", 1, id, Some(score), "because", outcome, "2026-09-09T00:00:00Z",
        )
        .unwrap();
    }

    #[test]
    fn exact_key_hits() {
        let conn = db();
        let id = identity("p1", "v1", "m1");
        seed(&conn, &id, 8, "inbox");

        let hit = lookup_exact(&conn, "hash-a", 1, &id).unwrap().unwrap();
        assert_eq!(hit.score, Some(8));
        assert_eq!(hit.outcome, "inbox");
    }

    #[test]
    fn a_different_pass_is_a_different_cache_entry() {
        let conn = db();
        let id = identity("p1", "v1", "m1");
        seed(&conn, &id, 8, "inbox");
        assert!(lookup_exact(&conn, "hash-a", 2, &id).unwrap().is_none());
    }

    #[test]
    fn profile_content_change_misses_and_forces_a_rescore() {
        let conn = db();
        seed(&conn, &identity("p1", "v1", "m1"), 3, "under_cutoff");

        // Same listing, same prompt, same model — but the CV was edited.
        assert!(lookup_reusable(&conn, "hash-a", 1, "p2-edited", "v1")
            .unwrap()
            .is_none());
    }

    #[test]
    fn prompt_version_change_misses_and_forces_a_rescore() {
        let conn = db();
        seed(&conn, &identity("p1", "v1", "m1"), 3, "under_cutoff");
        assert!(lookup_reusable(&conn, "hash-a", 1, "p1", "v2")
            .unwrap()
            .is_none());
    }

    #[test]
    fn model_change_alone_still_reuses_the_existing_score() {
        let conn = db();
        seed(&conn, &identity("p1", "v1", "m1"), 3, "under_cutoff");

        // Provider switch. The exact cache misses...
        assert!(lookup_exact(&conn, "hash-a", 1, &identity("p1", "v1", "m2-new"))
            .unwrap()
            .is_none());
        // ...but the re-score policy reuses it, so the backlog is not re-spent.
        let reused = lookup_reusable(&conn, "hash-a", 1, "p1", "v1").unwrap().unwrap();
        assert_eq!(reused.score, Some(3));
    }

    #[test]
    fn invalid_scores_are_cached_as_null_rather_than_re_paid_for() {
        let conn = db();
        let id = identity("p1", "v1", "m1");
        record_sighting(&conn, "r1", "fp1", "hash-b", 1, &id, None, "invalid", "inbox", "t")
            .unwrap();

        let hit = lookup_exact(&conn, "hash-b", 1, &id).unwrap().unwrap();
        assert_eq!(hit.score, None);
    }

    #[test]
    fn re_recording_the_same_key_updates_rather_than_duplicating() {
        let conn = db();
        let id = identity("p1", "v1", "m1");
        seed(&conn, &id, 3, "under_cutoff");
        seed(&conn, &id, 9, "inbox");

        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_scored_sightings WHERE listing_content_hash = 'hash-a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(lookup_exact(&conn, "hash-a", 1, &id).unwrap().unwrap().score, Some(9));
    }

    #[test]
    fn backlog_count_reports_distinct_under_cutoff_listings() {
        let conn = db();
        seed(&conn, &identity("p1", "v1", "m1"), 3, "under_cutoff");
        record_sighting(
            &conn, "r1", "fp1", "hash-c", 1, &identity("p1", "v1", "m1"),
            Some(9), "good", "inbox", "t",
        )
        .unwrap();
        assert_eq!(count_under_cutoff(&conn).unwrap(), 1);
    }
}
