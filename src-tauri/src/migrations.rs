//! Versioned SQLite migrations driven by `PRAGMA user_version`.
//!
//! Baseline detection (H7): when `user_version` is 0, probe for the `jobs` table.
//! Present ⇒ stamp the baseline version without re-running DDL.
//! Absent ⇒ run `m0001` fully.

use rusqlite::{Connection, OptionalExtension};

/// Latest schema version applied by this module.
pub const LATEST_VERSION: i32 = 1;

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

/// Apply all pending migrations. Idempotent.
pub fn run(conn: &mut Connection) -> Result<(), String> {
    let version = user_version(conn)?;

    if version == 0 {
        if table_exists(conn, "jobs")? {
            // Existing install predating user_version: bring columns current, then stamp.
            add_missing_jobs_columns(conn)?;
            migrate_pdf_path_to_documents(conn)?;
            set_user_version(conn, LATEST_VERSION)?;
            return Ok(());
        }
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        m0001_baseline(&tx)?;
        tx.pragma_update(None, "user_version", LATEST_VERSION)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        return Ok(());
    }

    if version > LATEST_VERSION {
        return Err(format!(
            "Database user_version {version} is newer than supported {LATEST_VERSION}"
        ));
    }

    // Future migrations: for v in (version+1)..=LATEST { ... }
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
    conn.execute(&format!("VACUUM INTO '{}'", dest_str.replace('\'', "''")), [])
        .map_err(|e| format!("VACUUM INTO failed: {e}"))?;
    Ok(())
}

/// Parse job column names from the TypeScript schema mirror file.
#[cfg_attr(not(test), allow(dead_code))]
pub fn parse_schema_ts_job_columns(schema_ts: &str) -> Result<Vec<String>, String> {
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
    let body = &rest[..end];
    let mut cols = Vec::new();
    for part in body.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let name = t.trim_matches('"').trim_matches('\'').trim();
        if !name.is_empty() {
            cols.push(name.to_string());
        }
    }
    if cols.is_empty() {
        return Err("schema.ts: jobs array empty".into());
    }
    Ok(cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

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
