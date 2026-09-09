//! §5.1 clustering: near-duplicates, weak→strong promotion, alias resolution.

#![allow(dead_code)] // dismiss/restore are exercised in tests; UI commands land in PR C.

use rusqlite::{Connection, OptionalExtension, params};

/// Resolve a fingerprint id through aliases (weak id → canonical).
pub fn resolve_id(conn: &Connection, fingerprint_id: &str) -> Result<String, String> {
    let canonical: Option<String> = conn
        .query_row(
            "SELECT fingerprint_id FROM mail_fingerprint_aliases WHERE alias_id = ?1",
            params![fingerprint_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(canonical.unwrap_or_else(|| fingerprint_id.to_string()))
}

/// Decide the cluster id for a listing and apply promotion / near-dup detection.
///
/// Returns `(fingerprint_id, near_duplicate_of)`.
pub fn upsert_cluster(
    conn: &Connection,
    strong: Option<&str>,
    weak: &str,
    now: &str,
) -> Result<(String, Option<String>), String> {
    let weak_id = format!("weak:{weak}");
    let target_id = match strong {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => weak_id.clone(),
    };

    // Promotion: weak-only cluster gains a strong key (§5.1 rule 4).
    if let Some(s) = strong.filter(|s| !s.is_empty()) {
        let weak_row: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT fingerprint_id, strong_key FROM mail_fingerprints WHERE weak_key = ?1 AND fingerprint_id = ?2",
                params![weak, &weak_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some((old_id, old_strong)) = weak_row {
            if old_strong.is_none() && old_id != s {
                promote_weak_to_strong(conn, &old_id, s, weak, now)?;
                return Ok((s.to_string(), None));
            }
        }
    }

    // Weak sighting after an existing strong cluster with the same weak key → join it (§5.1 rule 4).
    let target_id = if strong.filter(|s| !s.is_empty()).is_none() {
        let existing_strong: Option<String> = conn
            .query_row(
                "SELECT fingerprint_id FROM mail_fingerprints
                 WHERE weak_key = ?1 AND strong_key IS NOT NULL
                 ORDER BY first_seen_at ASC LIMIT 1",
                params![weak],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        existing_strong.unwrap_or(target_id)
    } else {
        target_id
    };

    // Upsert the target cluster row.
    conn.execute(
        "INSERT INTO mail_fingerprints (
            fingerprint_id, strong_key, weak_key, first_seen_at, last_seen_at, seen_count
         ) VALUES (?1, ?2, ?3, ?4, ?4, 1)
         ON CONFLICT(fingerprint_id) DO UPDATE SET
            last_seen_at = excluded.last_seen_at,
            seen_count = seen_count + 1,
            strong_key = COALESCE(excluded.strong_key, mail_fingerprints.strong_key)",
        params![
            &target_id,
            strong.filter(|s| !s.is_empty()),
            weak,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    // Near-duplicate: same weak key, different strong keys.
    let near = if strong.filter(|s| !s.is_empty()).is_some() {
        find_near_duplicate(conn, weak, &target_id)?
    } else {
        None
    };

    Ok((target_id, near))
}

fn promote_weak_to_strong(
    conn: &Connection,
    old_id: &str,
    strong: &str,
    weak: &str,
    now: &str,
) -> Result<(), String> {
    // Insert canonical strong row (may already exist from a race — ignore conflict).
    conn.execute(
        "INSERT INTO mail_fingerprints (
            fingerprint_id, strong_key, weak_key, first_seen_at, last_seen_at, seen_count
         ) VALUES (?1, ?1, ?2, ?3, ?3, 1)
         ON CONFLICT(fingerprint_id) DO UPDATE SET
            last_seen_at = excluded.last_seen_at,
            seen_count = seen_count + 1,
            strong_key = COALESCE(excluded.strong_key, mail_fingerprints.strong_key)",
        params![strong, weak, now],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT INTO mail_fingerprint_aliases (alias_id, fingerprint_id, created_at)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(alias_id) DO UPDATE SET fingerprint_id = excluded.fingerprint_id",
        params![old_id, strong, now],
    )
    .map_err(|e| e.to_string())?;

    // Point dismissals / inbox / sightings at the canonical id.
    for (table, col) in [
        ("mail_match_dismissals", "fingerprint_id"),
        ("mail_match_inbox", "fingerprint_id"),
        ("mail_scored_sightings", "fingerprint_id"),
    ] {
        conn.execute(
            &format!("UPDATE {table} SET {col} = ?1 WHERE {col} = ?2"),
            params![strong, old_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // near_duplicate_of may still reference the old id
    conn.execute(
        "UPDATE mail_match_inbox SET near_duplicate_of = ?1 WHERE near_duplicate_of = ?2",
        params![strong, old_id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM mail_fingerprints WHERE fingerprint_id = ?1",
        params![old_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn find_near_duplicate(
    conn: &Connection,
    weak: &str,
    self_id: &str,
) -> Result<Option<String>, String> {
    let other: Option<String> = conn
        .query_row(
            "SELECT fingerprint_id FROM mail_fingerprints
             WHERE weak_key = ?1 AND fingerprint_id != ?2 AND strong_key IS NOT NULL
             ORDER BY first_seen_at ASC LIMIT 1",
            params![weak, self_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(other)
}

pub fn dismiss(
    conn: &mut Connection,
    fingerprint_id: &str,
    run_id: Option<&str>,
    reason: Option<&str>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let canonical = resolve_id(conn, fingerprint_id)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO mail_match_dismissals (fingerprint_id, scope, reason, dismissed_at, dismissed_run)
         VALUES (?1, 'listing', ?2, ?3, ?4)
         ON CONFLICT(fingerprint_id) DO UPDATE SET
            reason = excluded.reason,
            dismissed_at = excluded.dismissed_at,
            dismissed_run = excluded.dismissed_run",
        params![canonical, reason, now, run_id],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE mail_match_inbox SET status = 'dismissed', updated_at = ?1
         WHERE fingerprint_id = ?2 AND status = 'pending'",
        params![now, canonical],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn restore(conn: &mut Connection, fingerprint_id: &str) -> Result<(), String> {
    let canonical = resolve_id(conn, fingerprint_id)?;
    conn.execute(
        "DELETE FROM mail_match_dismissals WHERE fingerprint_id = ?1",
        params![canonical],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn is_dismissed(conn: &Connection, fingerprint_id: &str) -> Result<bool, String> {
    let canonical = resolve_id(conn, fingerprint_id)?;
    let found: Option<String> = conn
        .query_row(
            "SELECT fingerprint_id FROM mail_match_dismissals WHERE fingerprint_id = ?1",
            params![canonical],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(found.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn setup() -> rusqlite::Connection {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();
        conn.execute(
            "INSERT INTO mail_scan_runs (run_id, status, started_at) VALUES ('r1', 'running', 't')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn different_strong_same_weak_are_near_duplicates() {
        let conn = setup();
        let now = "t";
        let (a, near_a) = upsert_cluster(&conn, Some("indeed:a"), "acme|dev|", now).unwrap();
        let (b, near_b) = upsert_cluster(&conn, Some("indeed:b"), "acme|dev|", now).unwrap();
        assert_eq!(a, "indeed:a");
        assert_eq!(b, "indeed:b");
        assert_ne!(a, b);
        assert!(near_a.is_none() || near_b.is_some());
        assert_eq!(near_b.as_deref(), Some("indeed:a"));
    }

    #[test]
    fn weak_after_strong_joins_existing_cluster() {
        let conn = setup();
        let now = "t";
        let (strong_id, _) =
            upsert_cluster(&conn, Some("indeed:first"), "same|role|", now).unwrap();
        let (joined, near) = upsert_cluster(&conn, None, "same|role|", now).unwrap();
        assert_eq!(joined, strong_id);
        assert!(near.is_none());
        let weak_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mail_fingerprints WHERE fingerprint_id LIKE 'weak:%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(weak_rows, 0);
    }

    #[test]
    fn weak_only_promotes_to_strong_with_alias_and_dismissal_follows() {
        let mut conn = setup();
        let now = "t";
        let (weak_id, _) = upsert_cluster(&conn, None, "solo|design|", now).unwrap();
        assert_eq!(weak_id, "weak:solo|design|");
        dismiss(&mut conn, &weak_id, Some("r1"), Some("nope")).unwrap();
        assert!(is_dismissed(&conn, &weak_id).unwrap());

        let (strong_id, _) =
            upsert_cluster(&conn, Some("indeed:xyz"), "solo|design|", now).unwrap();
        assert_eq!(strong_id, "indeed:xyz");

        let alias: String = conn
            .query_row(
                "SELECT fingerprint_id FROM mail_fingerprint_aliases WHERE alias_id = ?1",
                params![weak_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(alias, "indeed:xyz");

        // Dismissal followed the alias / migration.
        assert!(is_dismissed(&conn, "indeed:xyz").unwrap());
        assert!(is_dismissed(&conn, &weak_id).unwrap()); // resolve via alias
    }
}
