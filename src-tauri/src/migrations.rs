//! Versioned SQLite migrations driven by `PRAGMA user_version`.
//!
//! To add a migration: write an idempotent `fn(&Connection) -> Result<(), String>`
//! and append a [`Migration`] to [`MIGRATIONS`]. [`LATEST_VERSION`] follows.
//!
//! Installs predating `user_version` read as version 0 while already holding the
//! baseline schema (H7). Rather than probing the schema to distinguish them, every
//! migration is written to be idempotent, so re-applying `m0001` to such a database
//! is a no-op and fresh/existing installs converge on the same schema.

use rusqlite::{Connection, OptionalExtension};

/// One forward-only schema step. `up` must be idempotent.
struct Migration {
    version: i32,
    up: fn(&Connection) -> Result<(), String>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        up: m0001_baseline,
    },
    Migration {
        version: 2,
        up: m0002_mail_scan_core,
    },
    Migration {
        version: 3,
        up: m0003_mail_scan_indices,
    },
];

/// Latest schema version applied by this module.
pub const LATEST_VERSION: i32 = MIGRATIONS[MIGRATIONS.len() - 1].version;

const BASELINE_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company TEXT NOT NULL,
  title TEXT,
  url TEXT,
  raw_text TEXT,
  status TEXT NOT NULL,
  deadline TEXT,
  interview_date TEXT,
  start_date TEXT,
  tags TEXT,
  detected_language TEXT,
  notes TEXT,
  pdf_path TEXT,
  contact_name TEXT,
  contact_email TEXT,
  contact_phone TEXT,
  workplace_street TEXT,
  workplace_city TEXT,
  workplace_postal_code TEXT,
  work_mode TEXT,
  salary_range TEXT,
  contract_type TEXT,
  priority INTEGER,
  reference_number TEXT,
  source TEXT,
  listing_status TEXT,
  listing_checked_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS status_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id INTEGER NOT NULL,
  from_status TEXT,
  to_status TEXT NOT NULL,
  changed_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS job_documents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id INTEGER NOT NULL,
  doc_type TEXT NOT NULL,
  original_name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_jobs_updated_at ON jobs(updated_at);
CREATE INDEX IF NOT EXISTS idx_status_history_job_changed
  ON status_history(job_id, changed_at);
CREATE INDEX IF NOT EXISTS idx_job_documents_job_created
  ON job_documents(job_id, created_at);
"#;

/// Columns that older installs may lack; added idempotently in m0001.
const JOBS_EXTRA_COLUMNS: &[(&str, &str)] = &[
    ("interview_date", "TEXT"),
    ("start_date", "TEXT"),
    ("contact_name", "TEXT"),
    ("contact_email", "TEXT"),
    ("contact_phone", "TEXT"),
    ("workplace_street", "TEXT"),
    ("workplace_city", "TEXT"),
    ("workplace_postal_code", "TEXT"),
    ("work_mode", "TEXT"),
    ("salary_range", "TEXT"),
    ("contract_type", "TEXT"),
    ("priority", "INTEGER"),
    ("reference_number", "TEXT"),
    ("source", "TEXT"),
    ("listing_status", "TEXT"),
    ("listing_checked_at", "TEXT"),
];

/// Per-connection pragmas. `foreign_keys` must be set every time (not persistent).
///
/// `journal_mode = WAL` is a **persistent database property** and is set separately
/// via [`ensure_wal`] on first open.
pub fn apply_connection_pragmas(conn: &Connection) -> Result<(), String> {
    conn.pragma_update(None, "busy_timeout", 5000_i32)
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", true)
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Set WAL once. Safe to call repeatedly.
pub fn ensure_wal(conn: &Connection) -> Result<(), String> {
    // journal_mode returns the mode string; ignore value after setting.
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Failed to enable WAL: {e}"))?;
    Ok(())
}

fn user_version(conn: &Connection) -> Result<i32, String> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())
}

fn set_user_version(conn: &Connection, version: i32) -> Result<(), String> {
    conn.pragma_update(None, "user_version", version)
        .map_err(|e| e.to_string())
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool, String> {
    let found: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(found.is_some())
}

pub fn jobs_column_names(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(jobs)")
        .map_err(|e| e.to_string())?;
    let cols = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(cols)
}

fn add_missing_jobs_columns(conn: &Connection) -> Result<(), String> {
    let cols = jobs_column_names(conn)?;
    for (col, col_type) in JOBS_EXTRA_COLUMNS {
        if !cols.iter().any(|c| c == col) {
            conn.execute(&format!("ALTER TABLE jobs ADD COLUMN {col} {col_type}"), [])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn migrate_pdf_path_to_documents(conn: &Connection) -> Result<(), String> {
    if !table_exists(conn, "jobs")? || !table_exists(conn, "job_documents")? {
        return Ok(());
    }
    let has_pdf: bool = jobs_column_names(conn)?.iter().any(|c| c == "pdf_path");
    if !has_pdf {
        return Ok(());
    }
    let rows: Vec<(i64, String)> = {
        let mut stmt = conn
            .prepare("SELECT id, pdf_path FROM jobs WHERE pdf_path IS NOT NULL")
            .map_err(|e| e.to_string())?;
        let mapped = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let now = chrono::Utc::now().to_rfc3339();
    for (job_id, file_path) in rows {
        let already: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_documents WHERE job_id = ?1 AND file_path = ?2",
                rusqlite::params![job_id, &file_path],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if already == 0 {
            let original_name = std::path::Path::new(&file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| file_path.clone());
            conn.execute(
                "INSERT INTO job_documents (job_id, doc_type, original_name, file_path, created_at) VALUES (?1, 'other', ?2, ?3, ?4)",
                rusqlite::params![job_id, original_name, file_path, now],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn m0001_baseline(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(BASELINE_DDL)
        .map_err(|e| format!("m0001 DDL failed: {e}"))?;
    add_missing_jobs_columns(conn)?;
    migrate_pdf_path_to_documents(conn)?;
    Ok(())
}

const MAIL_SCAN_CORE_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS mail_scan_runs (
  run_id        TEXT PRIMARY KEY,
  trigger       TEXT NOT NULL DEFAULT 'manual',
  status        TEXT NOT NULL CHECK (status IN ('running','completed','cancelled','failed')),
  started_at    TEXT NOT NULL,
  finished_at   TEXT,
  stats_json    TEXT NOT NULL DEFAULT '{}',
  error_code    TEXT,
  error_summary TEXT,
  sidecar_version TEXT,
  model_id      TEXT,
  profile_short_hash TEXT,
  profile_full_hash  TEXT
);

CREATE TABLE IF NOT EXISTS mail_fingerprints (
  fingerprint_id TEXT PRIMARY KEY,
  strong_key     TEXT UNIQUE,
  weak_key       TEXT NOT NULL,
  first_seen_at  TEXT NOT NULL,
  last_seen_at   TEXT NOT NULL,
  seen_count     INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS mail_fingerprint_aliases (
  alias_id       TEXT PRIMARY KEY,
  fingerprint_id TEXT NOT NULL REFERENCES mail_fingerprints(fingerprint_id) ON DELETE CASCADE,
  created_at     TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mail_match_inbox (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  fingerprint_id    TEXT NOT NULL REFERENCES mail_fingerprints(fingerprint_id) ON DELETE CASCADE,
  kind              TEXT NOT NULL CHECK (kind IN ('new','update_suggestion')),
  status            TEXT NOT NULL CHECK (status IN ('pending','accepted','dismissed','superseded')),
  job_id            INTEGER REFERENCES jobs(id) ON DELETE CASCADE,
  score             INTEGER CHECK (score BETWEEN 0 AND 10),
  score_reason      TEXT,
  score_state       TEXT NOT NULL CHECK (score_state IN ('ok','invalid','skipped')),
  suspicious        INTEGER NOT NULL DEFAULT 0,
  near_duplicate_of TEXT REFERENCES mail_fingerprints(fingerprint_id),
  draft_json        TEXT NOT NULL,
  enrichment_state  TEXT NOT NULL CHECK (enrichment_state IN ('complete','partial','failed','skipped')),
  enrichment_error  TEXT,
  source_board      TEXT,
  message_id        TEXT,
  message_date      TEXT,
  listing_url       TEXT,
  base_job_updated_at TEXT,
  first_run_id      TEXT NOT NULL REFERENCES mail_scan_runs(run_id),
  last_run_id       TEXT NOT NULL REFERENCES mail_scan_runs(run_id),
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL,
  title             TEXT GENERATED ALWAYS AS (json_extract(draft_json,'$.title')) VIRTUAL,
  company           TEXT GENERATED ALWAYS AS (json_extract(draft_json,'$.company')) VIRTUAL
);

CREATE TABLE IF NOT EXISTS mail_match_dismissals (
  fingerprint_id TEXT PRIMARY KEY REFERENCES mail_fingerprints(fingerprint_id) ON DELETE CASCADE,
  scope          TEXT NOT NULL DEFAULT 'listing',
  reason         TEXT,
  dismissed_at   TEXT NOT NULL,
  dismissed_run  TEXT
);

CREATE TABLE IF NOT EXISTS mail_scored_sightings (
  id                  INTEGER PRIMARY KEY AUTOINCREMENT,
  fingerprint_id      TEXT NOT NULL REFERENCES mail_fingerprints(fingerprint_id) ON DELETE CASCADE,
  pass                INTEGER NOT NULL CHECK (pass IN (1,2)),
  score               INTEGER,
  reason              TEXT,
  profile_hash        TEXT NOT NULL,
  prompt_version      TEXT NOT NULL,
  model_id            TEXT NOT NULL,
  listing_content_hash TEXT NOT NULL,
  outcome             TEXT NOT NULL CHECK (outcome IN ('inbox','under_cutoff','flagged_irrelevant','error')),
  scored_at           TEXT NOT NULL,
  run_id              TEXT NOT NULL REFERENCES mail_scan_runs(run_id)
);

CREATE TABLE IF NOT EXISTS mail_source_cursors (
  source_id       TEXT PRIMARY KEY,
  path            TEXT NOT NULL,
  kind            TEXT NOT NULL,
  size            INTEGER,
  mtime_ns        INTEGER,
  offset          INTEGER,
  sentinel_hash   TEXT,
  last_message_id TEXT,
  updated_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS job_field_provenance (
  job_id     INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  field      TEXT NOT NULL,
  source     TEXT NOT NULL,
  run_id     TEXT,
  set_at     TEXT NOT NULL,
  PRIMARY KEY (job_id, field, set_at)
);
"#;

const MAIL_SCAN_JOB_COLUMNS: &[(&str, &str)] = &[
    ("mail_score", "INTEGER"),
    ("mail_score_reason", "TEXT"),
    ("mail_scored_at", "TEXT"),
];

fn add_mail_score_columns(conn: &Connection) -> Result<(), String> {
    let cols = jobs_column_names(conn)?;
    for (col, col_type) in MAIL_SCAN_JOB_COLUMNS {
        if !cols.iter().any(|c| c == col) {
            conn.execute(&format!("ALTER TABLE jobs ADD COLUMN {col} {col_type}"), [])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn m0002_mail_scan_core(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(MAIL_SCAN_CORE_DDL)
        .map_err(|e| format!("m0002 DDL failed: {e}"))?;
    add_mail_score_columns(conn)?;
    Ok(())
}

const MAIL_SCAN_INDEX_DDL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_mail_fp_weak ON mail_fingerprints(weak_key);
CREATE UNIQUE INDEX IF NOT EXISTS idx_mmi_one_pending
  ON mail_match_inbox(fingerprint_id, kind) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_mmi_pending ON mail_match_inbox(status, score DESC, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_mmi_job ON mail_match_inbox(job_id) WHERE job_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_sighting_cache
  ON mail_scored_sightings(listing_content_hash, pass, profile_hash, prompt_version, model_id);
CREATE INDEX IF NOT EXISTS idx_sighting_fp ON mail_scored_sightings(fingerprint_id, pass, scored_at DESC);
CREATE INDEX IF NOT EXISTS idx_runs_started ON mail_scan_runs(started_at DESC);
"#;

fn m0003_mail_scan_indices(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(MAIL_SCAN_INDEX_DDL)
        .map_err(|e| format!("m0003 DDL failed: {e}"))?;
    Ok(())
}

/// Apply every migration newer than the database's `user_version`.
///
/// Each step runs in its own transaction and stamps `user_version` on commit, so a
/// failure leaves the database at the last successfully applied version.
pub fn run(conn: &mut Connection) -> Result<(), String> {
    let version = user_version(conn)?;
    if version > LATEST_VERSION {
        return Err(format!(
            "Database user_version {version} is newer than supported {LATEST_VERSION}"
        ));
    }

    for migration in MIGRATIONS.iter().filter(|m| m.version > version) {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        (migration.up)(&tx)?;
        set_user_version(&tx, migration.version)?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Consistent single-file backup that works under WAL (no `-wal`/`-shm` sidecars).
pub fn vacuum_into(conn: &Connection, dest: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if dest.exists() {
        std::fs::remove_file(dest).map_err(|e| e.to_string())?;
    }
    let dest_str = dest.to_str().ok_or_else(|| "Backup path is not UTF-8".to_string())?;
    conn.execute("VACUUM INTO ?1", [dest_str])
        .map_err(|e| format!("VACUUM INTO failed: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Parse job column names out of the TypeScript schema mirror file.
    fn parse_schema_ts_job_columns(schema_ts: &str) -> Result<Vec<String>, String> {
        let start = schema_ts
            .find("jobs:")
            .ok_or_else(|| "schema.ts: missing jobs: key".to_string())?;
        let after = &schema_ts[start..];
        let bracket = after
            .find('[')
            .ok_or_else(|| "schema.ts: missing jobs array".to_string())?;
        let rest = &after[bracket + 1..];
        let end = rest
            .find(']')
            .ok_or_else(|| "schema.ts: unclosed jobs array".to_string())?;
        let cols: Vec<String> = rest[..end]
            .split(',')
            .map(|part| part.trim().trim_matches(['"', '\'']).trim().to_string())
            .filter(|name| !name.is_empty())
            .collect();
        if cols.is_empty() {
            return Err("schema.ts: jobs array empty".into());
        }
        Ok(cols)
    }

    fn open_mem() -> Connection {
        Connection::open_in_memory().expect("mem db")
    }

    fn shape_like_released_schema(conn: &Connection) {
        // Mimic today's released DB: CREATE without listing_* then ALTER-style columns.
        conn.execute_batch(
            r#"
            CREATE TABLE jobs (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              company TEXT NOT NULL,
              title TEXT,
              url TEXT,
              raw_text TEXT,
              status TEXT NOT NULL,
              deadline TEXT,
              interview_date TEXT,
              start_date TEXT,
              tags TEXT,
              detected_language TEXT,
              notes TEXT,
              pdf_path TEXT,
              contact_name TEXT,
              contact_email TEXT,
              contact_phone TEXT,
              workplace_street TEXT,
              workplace_city TEXT,
              workplace_postal_code TEXT,
              work_mode TEXT,
              salary_range TEXT,
              contract_type TEXT,
              priority INTEGER,
              reference_number TEXT,
              source TEXT,
              listing_status TEXT,
              listing_checked_at TEXT,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            CREATE TABLE status_history (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              job_id INTEGER NOT NULL,
              from_status TEXT,
              to_status TEXT NOT NULL,
              changed_at TEXT NOT NULL
            );
            CREATE TABLE job_documents (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              job_id INTEGER NOT NULL,
              doc_type TEXT NOT NULL,
              original_name TEXT NOT NULL,
              file_path TEXT NOT NULL,
              created_at TEXT NOT NULL
            );
            "#,
        )
        .unwrap();
        assert_eq!(user_version(conn).unwrap(), 0);
    }

    #[test]
    fn migrates_existing_zero_version_db_idempotently() {
        let mut conn = open_mem();
        shape_like_released_schema(&conn);
        run(&mut conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), LATEST_VERSION);
        let cols_first: BTreeSet<_> = jobs_column_names(&conn).unwrap().into_iter().collect();
        assert!(cols_first.contains("listing_status"));
        assert!(cols_first.contains("listing_checked_at"));

        run(&mut conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), LATEST_VERSION);
        let cols_second: BTreeSet<_> = jobs_column_names(&conn).unwrap().into_iter().collect();
        assert_eq!(cols_first, cols_second);
    }

    #[test]
    fn fresh_and_existing_converge_on_same_jobs_columns() {
        let mut existing = open_mem();
        shape_like_released_schema(&existing);
        run(&mut existing).unwrap();

        let mut fresh = open_mem();
        run(&mut fresh).unwrap();

        let a: BTreeSet<_> = jobs_column_names(&existing).unwrap().into_iter().collect();
        let b: BTreeSet<_> = jobs_column_names(&fresh).unwrap().into_iter().collect();
        assert_eq!(a, b);
        assert_eq!(user_version(&existing).unwrap(), user_version(&fresh).unwrap());
    }

    #[test]
    fn schema_ts_mirror_lists_mail_tables() {
        let schema_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/db/schema.ts");
        let schema_ts = std::fs::read_to_string(schema_path).expect("read schema.ts");
        for table in [
            "mail_scan_runs",
            "mail_fingerprints",
            "mail_fingerprint_aliases",
            "mail_match_inbox",
            "mail_match_dismissals",
            "mail_scored_sightings",
            "mail_source_cursors",
            "job_field_provenance",
        ] {
            assert!(
                schema_ts.contains(&format!("{table}:")),
                "schema.ts missing table key {table}"
            );
        }
    }

    #[test]
    fn schema_ts_mirror_lists_every_jobs_column() {
        let mut conn = open_mem();
        run(&mut conn).unwrap();
        let db_cols: BTreeSet<_> = jobs_column_names(&conn).unwrap().into_iter().collect();

        let schema_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/db/schema.ts");
        let schema_ts = std::fs::read_to_string(schema_path).expect("read schema.ts");
        let mirror: BTreeSet<_> = parse_schema_ts_job_columns(&schema_ts)
            .unwrap()
            .into_iter()
            .collect();

        let missing: Vec<_> = db_cols.difference(&mirror).cloned().collect();
        assert!(
            missing.is_empty(),
            "schema.ts missing jobs columns: {missing:?}"
        );
    }

    #[test]
    fn migration_versions_are_contiguous_and_ascending() {
        for (index, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(
                migration.version,
                index as i32 + 1,
                "MIGRATIONS must be ordered and start at 1 with no gaps, \
                 otherwise run() silently skips a step"
            );
        }
        assert_eq!(LATEST_VERSION, MIGRATIONS.len() as i32);
    }

    /// Opt-in check against a copy of a real install, since fixtures can only model the
    /// schema we *think* shipped. Run with:
    /// `JOBTRACKER_REAL_DB=/path/to/copy.db cargo test --ignored migrates_a_real_database`
    #[test]
    #[ignore = "set JOBTRACKER_REAL_DB to a copy of a real database"]
    fn migrates_a_real_database_copy() {
        let path = std::env::var("JOBTRACKER_REAL_DB").expect("JOBTRACKER_REAL_DB not set");
        let mut conn = Connection::open(&path).unwrap();
        let count = |c: &Connection| -> i64 {
            c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0)).unwrap()
        };
        let before = count(&conn);

        ensure_wal(&conn).unwrap();
        apply_connection_pragmas(&conn).unwrap();
        run(&mut conn).unwrap();

        assert_eq!(user_version(&conn).unwrap(), LATEST_VERSION);
        assert_eq!(count(&conn), before, "migration must not lose or duplicate jobs");
        let cols: BTreeSet<_> = jobs_column_names(&conn).unwrap().into_iter().collect();
        assert!(cols.contains("listing_status"));
        assert!(cols.contains("listing_checked_at"));
        assert!(cols.contains("mail_score"));

        run(&mut conn).unwrap();
        assert_eq!(count(&conn), before);
        let cols_again: BTreeSet<_> = jobs_column_names(&conn).unwrap().into_iter().collect();
        assert_eq!(cols, cols_again, "second run must change nothing");
    }

    #[test]
    fn m0002_creates_mail_tables_and_is_idempotent() {
        let mut conn = open_mem();
        run(&mut conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), LATEST_VERSION);
        for table in [
            "mail_scan_runs",
            "mail_fingerprints",
            "mail_fingerprint_aliases",
            "mail_match_inbox",
            "mail_match_dismissals",
            "mail_scored_sightings",
            "mail_source_cursors",
            "job_field_provenance",
        ] {
            assert!(table_exists(&conn, table).unwrap(), "missing {table}");
        }
        let cols: BTreeSet<_> = jobs_column_names(&conn).unwrap().into_iter().collect();
        assert!(cols.contains("mail_score"));
        assert!(cols.contains("mail_score_reason"));
        assert!(cols.contains("mail_scored_at"));

        // Re-running must be a no-op (user_version already at latest).
        run(&mut conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), LATEST_VERSION);
    }

    #[test]
    fn partial_unique_index_allows_accepted_plus_one_pending() {
        let mut conn = open_mem();
        apply_connection_pragmas(&conn).unwrap();
        run(&mut conn).unwrap();

        let now = "2026-09-09T12:00:00Z";
        conn.execute(
            "INSERT INTO mail_scan_runs (run_id, status, started_at) VALUES ('run1', 'running', ?1)",
            [now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mail_fingerprints (fingerprint_id, strong_key, weak_key, first_seen_at, last_seen_at)
             VALUES ('indeed:abc', 'indeed:abc', 'acme|dev|kbh', ?1, ?1)",
            [now],
        )
        .unwrap();

        let insert = |status: &str| {
            conn.execute(
                "INSERT INTO mail_match_inbox (
                    fingerprint_id, kind, status, score_state, draft_json,
                    enrichment_state, first_run_id, last_run_id, created_at, updated_at
                 ) VALUES ('indeed:abc', 'new', ?1, 'skipped', '{}', 'skipped', 'run1', 'run1', ?2, ?2)",
                rusqlite::params![status, now],
            )
        };

        insert("pending").unwrap();
        insert("accepted").unwrap();
        let second_pending = insert("pending");
        assert!(
            second_pending.is_err(),
            "second pending row must violate idx_mmi_one_pending"
        );
    }

    #[test]
    fn vacuum_into_preserves_rows_under_wal() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("app.db");
        let bak_path = dir.path().join("backup.db");

        {
            let mut conn = Connection::open(&db_path).unwrap();
            ensure_wal(&conn).unwrap();
            apply_connection_pragmas(&conn).unwrap();
            run(&mut conn).unwrap();
            conn.execute(
                "INSERT INTO jobs (company, status, created_at, updated_at) VALUES ('Acme', 'Interesting', 't', 't')",
                [],
            )
            .unwrap();
            // Do not checkpoint — leave content potentially in the WAL.
            vacuum_into(&conn, &bak_path).unwrap();
        }

        let bak = Connection::open(&bak_path).unwrap();
        let count: i64 = bak
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE company = 'Acme'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
