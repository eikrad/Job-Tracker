use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Serialize, Deserialize)]
pub struct Job {
  pub id: i64,
  pub company: String,
  pub title: Option<String>,
  pub url: Option<String>,
  pub raw_text: Option<String>,
  pub status: String,
  pub deadline: Option<String>,
  pub interview_date: Option<String>,
  pub start_date: Option<String>,
  pub tags: Option<String>,
  pub detected_language: Option<String>,
  pub notes: Option<String>,
  pub pdf_path: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct JobDocument {
  pub id: i64,
  pub job_id: i64,
  pub doc_type: String,
  pub original_name: String,
  pub file_path: String,
  pub created_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewJob {
  pub company: String,
  pub title: Option<String>,
  pub url: Option<String>,
  pub raw_text: Option<String>,
  pub status: String,
  pub deadline: Option<String>,
  pub interview_date: Option<String>,
  pub start_date: Option<String>,
  pub tags: Option<String>,
  pub detected_language: Option<String>,
  pub notes: Option<String>,
}

fn ensure_storage_dirs(base: &Path) -> Result<(), String> {
  fs::create_dir_all(base.join("data")).map_err(|e| e.to_string())?;
  fs::create_dir_all(base.join("storage").join("applications")).map_err(|e| e.to_string())?;
  Ok(())
}

fn db_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
  ensure_storage_dirs(&app_data_dir)?;
  Ok(app_data_dir.join("data").join("app.db"))
}

pub(crate) fn connection(app: &tauri::AppHandle) -> Result<Connection, String> {
  let path = db_path(app)?;
  Connection::open(path).map_err(|e| format!("DB open failed: {e}"))
}

const SQL_INSERT_JOB: &str = r#"
INSERT INTO jobs (company, title, url, raw_text, status, deadline, interview_date, start_date, tags, detected_language, notes, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
"#;

fn insert_new_job(conn: &Connection, payload: &NewJob, now: &str) -> Result<usize, rusqlite::Error> {
  conn.execute(
    SQL_INSERT_JOB,
    params![
      &payload.company,
      &payload.title,
      &payload.url,
      &payload.raw_text,
      &payload.status,
      &payload.deadline,
      &payload.interview_date,
      &payload.start_date,
      &payload.tags,
      &payload.detected_language,
      &payload.notes,
      now,
      now
    ],
  )
}

/// One-time migration: move existing pdf_path rows into job_documents as doc_type = 'other'.
fn migrate_pdf_path_to_documents(conn: &Connection) -> Result<(), String> {
  let rows: Vec<(i64, String)> = {
    let mut stmt = conn
      .prepare("SELECT id, pdf_path FROM jobs WHERE pdf_path IS NOT NULL")
      .map_err(|e| e.to_string())?;
    let rows = stmt
      .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
      .map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>()
      .map_err(|e| e.to_string())?;
    rows
  };
  let now = Utc::now().to_rfc3339();
  for (job_id, file_path) in rows {
    let already: i64 = conn
      .query_row(
        "SELECT COUNT(*) FROM job_documents WHERE job_id = ?1 AND file_path = ?2",
        params![job_id, &file_path],
        |r| r.get(0),
      )
      .map_err(|e| e.to_string())?;
    if already == 0 {
      let original_name = std::path::Path::new(&file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.clone());
      conn
        .execute(
          "INSERT INTO job_documents (job_id, doc_type, original_name, file_path, created_at) VALUES (?1, 'other', ?2, ?3, ?4)",
          params![job_id, original_name, file_path, now],
        )
        .map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

fn migrate_jobs_columns(conn: &Connection) -> Result<(), String> {
  let mut stmt = conn
    .prepare("PRAGMA table_info(jobs)")
    .map_err(|e| e.to_string())?;
  let cols: Vec<String> = stmt
    .query_map([], |row| row.get::<_, String>(1))
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
  if !cols.iter().any(|c| c == "interview_date") {
    conn
      .execute("ALTER TABLE jobs ADD COLUMN interview_date TEXT", [])
      .map_err(|e| e.to_string())?;
  }
  if !cols.iter().any(|c| c == "start_date") {
    conn
      .execute("ALTER TABLE jobs ADD COLUMN start_date TEXT", [])
      .map_err(|e| e.to_string())?;
  }
  Ok(())
}

#[tauri::command]
pub fn init_db(app: tauri::AppHandle) -> Result<(), String> {
  let conn = connection(&app)?;
  conn
    .execute_batch(
      r#"
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
      "#,
    )
    .map_err(|e| format!("DB init failed: {e}"))?;
  migrate_jobs_columns(&conn)?;
  migrate_pdf_path_to_documents(&conn)?;
  Ok(())
}

#[tauri::command]
pub fn create_job(app: tauri::AppHandle, payload: NewJob) -> Result<i64, String> {
  let conn = connection(&app)?;
  let now = Utc::now().to_rfc3339();
  insert_new_job(&conn, &payload, &now).map_err(|e| format!("Create job failed: {e}"))?;
  Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn list_jobs(app: tauri::AppHandle) -> Result<Vec<Job>, String> {
  let conn = connection(&app)?;
  let mut stmt = conn
    .prepare(
      "SELECT id, company, title, url, raw_text, status, deadline, interview_date, start_date, tags, detected_language, notes, pdf_path, created_at, updated_at
      FROM jobs ORDER BY updated_at DESC",
    )
    .map_err(|e| e.to_string())?;
  let jobs = stmt
    .query_map([], |row| {
      Ok(Job {
        id: row.get(0)?,
        company: row.get(1)?,
        title: row.get(2)?,
        url: row.get(3)?,
        raw_text: row.get(4)?,
        status: row.get(5)?,
        deadline: row.get(6)?,
        interview_date: row.get(7)?,
        start_date: row.get(8)?,
        tags: row.get(9)?,
        detected_language: row.get(10)?,
        notes: row.get(11)?,
        pdf_path: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
      })
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
  Ok(jobs)
}

/// Remove stored PDF only if it lives under `app_data/storage/applications` (safety).
fn remove_pdf_if_in_app_storage(app: &tauri::AppHandle, pdf_path: &str) -> Result<(), String> {
  let path = Path::new(pdf_path);
  if !path.is_file() {
    return Ok(());
  }
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
  let apps_dir = app_data_dir.join("storage").join("applications");
  let apps_canon = apps_dir.canonicalize().unwrap_or(apps_dir);
  let file_canon = match path.canonicalize() {
    Ok(c) => c,
    Err(_) => return Ok(()),
  };
  if file_canon.starts_with(&apps_canon) {
    let _ = fs::remove_file(file_canon);
  }
  Ok(())
}

#[tauri::command]
pub fn delete_job(app: tauri::AppHandle, job_id: i64) -> Result<(), String> {
  let mut conn = connection(&app)?;
  let tx = conn.transaction().map_err(|e| e.to_string())?;
  let pdf_path: Option<String> = match tx.query_row(
    "SELECT pdf_path FROM jobs WHERE id = ?1",
    params![job_id],
    |r| r.get::<_, Option<String>>(0),
  ) {
    Ok(p) => p,
    Err(rusqlite::Error::QueryReturnedNoRows) => return Err("Job not found.".to_string()),
    Err(e) => return Err(e.to_string()),
  };
  let doc_paths: Vec<String> = {
    let mut stmt = tx
      .prepare("SELECT file_path FROM job_documents WHERE job_id = ?1")
      .map_err(|e| e.to_string())?;
    let paths = stmt
      .query_map(params![job_id], |r| r.get::<_, String>(0))
      .map_err(|e| e.to_string())?
      .collect::<Result<Vec<_>, _>>()
      .map_err(|e| e.to_string())?;
    paths
  };
  tx.execute("DELETE FROM job_documents WHERE job_id = ?1", params![job_id])
    .map_err(|e| e.to_string())?;
  tx.execute(
    "DELETE FROM status_history WHERE job_id = ?1",
    params![job_id],
  )
  .map_err(|e| e.to_string())?;
  let n = tx
    .execute("DELETE FROM jobs WHERE id = ?1", params![job_id])
    .map_err(|e| e.to_string())?;
  if n == 0 {
    return Err("Job not found.".to_string());
  }
  tx.commit().map_err(|e| e.to_string())?;

  if let Some(ref p) = pdf_path {
    remove_pdf_if_in_app_storage(&app, p)?;
  }
  for p in &doc_paths {
    remove_pdf_if_in_app_storage(&app, p)?;
  }
  Ok(())
}

#[tauri::command]
pub fn update_job(app: tauri::AppHandle, job_id: i64, payload: NewJob) -> Result<(), String> {
  if payload.company.trim().is_empty() {
    return Err("Company is required.".to_string());
  }
  let conn = connection(&app)?;
  let old_status: String = conn
    .query_row("SELECT status FROM jobs WHERE id = ?1", params![job_id], |r| r.get(0))
    .map_err(|e| format!("Job not found: {e}"))?;
  let now = Utc::now().to_rfc3339();
  let n = conn
    .execute(
      "UPDATE jobs SET company = ?1, title = ?2, url = ?3, raw_text = ?4, status = ?5,
        deadline = ?6, interview_date = ?7, start_date = ?8, tags = ?9,
        detected_language = ?10, notes = ?11, updated_at = ?12
       WHERE id = ?13",
      params![
        payload.company.trim(),
        payload.title,
        payload.url,
        payload.raw_text,
        payload.status,
        payload.deadline,
        payload.interview_date,
        payload.start_date,
        payload.tags,
        payload.detected_language,
        payload.notes,
        now,
        job_id,
      ],
    )
    .map_err(|e| e.to_string())?;
  if n == 0 {
    return Err("Job not found.".to_string());
  }
  if old_status != payload.status {
    conn
      .execute(
        "INSERT INTO status_history (job_id, from_status, to_status, changed_at) VALUES (?1, ?2, ?3, ?4)",
        params![job_id, old_status, payload.status, now],
      )
      .map_err(|e| e.to_string())?;
  }
  Ok(())
}

#[tauri::command]
pub fn update_job_status(app: tauri::AppHandle, job_id: i64, new_status: String) -> Result<(), String> {
  let conn = connection(&app)?;
  let old_status: String = conn
    .query_row("SELECT status FROM jobs WHERE id = ?1", params![job_id], |r| r.get(0))
    .map_err(|e| format!("Fetch old status failed: {e}"))?;
  let now = Utc::now().to_rfc3339();
  conn
    .execute(
      "UPDATE jobs SET status = ?1, updated_at = ?2 WHERE id = ?3",
      params![new_status, now, job_id],
    )
    .map_err(|e| e.to_string())?;
  conn
    .execute(
      "INSERT INTO status_history (job_id, from_status, to_status, changed_at) VALUES (?1, ?2, ?3, ?4)",
      params![job_id, old_status, new_status, now],
    )
    .map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
pub fn list_status_history(app: tauri::AppHandle, job_id: i64) -> Result<Vec<serde_json::Value>, String> {
  let conn = connection(&app)?;
  let mut stmt = conn
    .prepare("SELECT from_status, to_status, changed_at FROM status_history WHERE job_id = ?1 ORDER BY changed_at DESC")
    .map_err(|e| e.to_string())?;
  let out = stmt
    .query_map(params![job_id], |row| {
      Ok(serde_json::json!({
        "from_status": row.get::<_, Option<String>>(0)?,
        "to_status": row.get::<_, String>(1)?,
        "changed_at": row.get::<_, String>(2)?,
      }))
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
  Ok(out)
}

#[tauri::command]
pub fn save_application_pdf(
  app: tauri::AppHandle,
  job_id: i64,
  original_name: String,
  bytes: Vec<u8>,
) -> Result<String, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
  ensure_storage_dirs(&app_data_dir)?;
  let safe_name = original_name
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
    .collect::<String>();
  let file_name = format!("{}_{}", job_id, safe_name);
  let target = app_data_dir.join("storage").join("applications").join(file_name);
  fs::write(&target, bytes).map_err(|e| format!("Write pdf failed: {e}"))?;

  let conn = connection(&app)?;
  conn
    .execute(
      "UPDATE jobs SET pdf_path = ?1, updated_at = ?2 WHERE id = ?3",
      params![target.to_string_lossy().to_string(), Utc::now().to_rfc3339(), job_id],
    )
    .map_err(|e| format!("Update pdf path failed: {e}"))?;
  Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_job_documents(app: tauri::AppHandle, job_id: i64) -> Result<Vec<JobDocument>, String> {
  let conn = connection(&app)?;
  let mut stmt = conn
    .prepare(
      "SELECT id, job_id, doc_type, original_name, file_path, created_at FROM job_documents WHERE job_id = ?1 ORDER BY created_at ASC",
    )
    .map_err(|e| e.to_string())?;
  let docs = stmt
    .query_map(params![job_id], |row| {
      Ok(JobDocument {
        id: row.get(0)?,
        job_id: row.get(1)?,
        doc_type: row.get(2)?,
        original_name: row.get(3)?,
        file_path: row.get(4)?,
        created_at: row.get(5)?,
      })
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
  Ok(docs)
}

#[tauri::command]
pub fn save_job_document(
  app: tauri::AppHandle,
  job_id: i64,
  doc_type: String,
  original_name: String,
  bytes: Vec<u8>,
) -> Result<JobDocument, String> {
  let app_data_dir = app
    .path()
    .app_data_dir()
    .map_err(|e| format!("Cannot resolve app data dir: {e}"))?;
  ensure_storage_dirs(&app_data_dir)?;
  let safe_name = original_name
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
    .collect::<String>();
  let file_name = format!("{}_{}_{}", job_id, doc_type, safe_name);
  let target = app_data_dir.join("storage").join("applications").join(&file_name);
  fs::write(&target, bytes).map_err(|e| format!("Write document failed: {e}"))?;

  let now = Utc::now().to_rfc3339();
  let file_path = target.to_string_lossy().to_string();
  let conn = connection(&app)?;
  conn
    .execute(
      "INSERT INTO job_documents (job_id, doc_type, original_name, file_path, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
      params![job_id, doc_type, original_name, file_path, now],
    )
    .map_err(|e| format!("Insert document failed: {e}"))?;
  let id = conn.last_insert_rowid();
  conn
    .execute(
      "UPDATE jobs SET updated_at = ?1 WHERE id = ?2",
      params![now, job_id],
    )
    .map_err(|e| e.to_string())?;
  Ok(JobDocument { id, job_id, doc_type, original_name, file_path, created_at: now })
}

#[tauri::command]
pub fn delete_job_document(app: tauri::AppHandle, doc_id: i64) -> Result<(), String> {
  let conn = connection(&app)?;
  let file_path: String = conn
    .query_row(
      "SELECT file_path FROM job_documents WHERE id = ?1",
      params![doc_id],
      |r| r.get(0),
    )
    .map_err(|e| format!("Document not found: {e}"))?;
  conn
    .execute("DELETE FROM job_documents WHERE id = ?1", params![doc_id])
    .map_err(|e| e.to_string())?;
  remove_pdf_if_in_app_storage(&app, &file_path)?;
  Ok(())
}

#[tauri::command]
pub fn backup_to_folder(dest: String, app: tauri::AppHandle) -> Result<(), String> {
  let app_data = app
    .path()
    .app_data_dir()
    .map_err(|e| e.to_string())?;
  let dest_path = std::path::PathBuf::from(shellexpand::tilde(&dest).as_ref());
  let dest_dir = dest_path.join("JobTracker");
  let dest_storage = dest_dir.join("storage").join("applications");

  std::fs::create_dir_all(&dest_storage).map_err(|e| e.to_string())?;

  let db_src = app_data.join("data").join("app.db");
  let db_dst = dest_dir.join("app.db");
  std::fs::copy(&db_src, &db_dst).map_err(|e| e.to_string())?;

  let pdf_src = app_data.join("storage").join("applications");
  if pdf_src.exists() {
    for entry in std::fs::read_dir(&pdf_src).map_err(|e| e.to_string())? {
      let entry = entry.map_err(|e| e.to_string())?;
      let fname = entry.file_name();
      std::fs::copy(entry.path(), dest_storage.join(&fname)).map_err(|e| e.to_string())?;
    }
  }
  Ok(())
}

pub(crate) fn is_importable_job(payload: &NewJob) -> bool {
  !payload.company.trim().is_empty()
}

#[tauri::command]
pub fn import_jobs(app: tauri::AppHandle, jobs: Vec<NewJob>) -> Result<usize, String> {
  let conn = connection(&app)?;
  let now = Utc::now().to_rfc3339();
  let mut count = 0usize;
  for payload in jobs {
    if !is_importable_job(&payload) {
      continue;
    }
    insert_new_job(&conn, &payload, &now).map_err(|e| format!("Import job failed: {e}"))?;
    count += 1;
  }
  Ok(count)
}

#[cfg(test)]
mod tests {
  use super::{is_importable_job, NewJob};

  fn sample_new_job(company: &str) -> NewJob {
    NewJob {
      company: company.to_string(),
      title: None,
      url: None,
      raw_text: None,
      status: "Interesting".to_string(),
      deadline: None,
      interview_date: None,
      start_date: None,
      tags: None,
      detected_language: None,
      notes: None,
    }
  }

  #[test]
  fn is_importable_job_skips_blank_company() {
    assert!(is_importable_job(&sample_new_job("Acme")));
    assert!(!is_importable_job(&sample_new_job("   ")));
    assert!(!is_importable_job(&sample_new_job("")));
  }
}
