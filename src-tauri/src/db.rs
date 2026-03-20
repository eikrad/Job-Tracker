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
  pub tags: Option<String>,
  pub detected_language: Option<String>,
  pub notes: Option<String>,
  pub pdf_path: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Serialize, Deserialize)]
pub struct NewJob {
  pub company: String,
  pub title: Option<String>,
  pub url: Option<String>,
  pub raw_text: Option<String>,
  pub status: String,
  pub deadline: Option<String>,
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
INSERT INTO jobs (company, title, url, raw_text, status, deadline, tags, detected_language, notes, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
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
      &payload.tags,
      &payload.detected_language,
      &payload.notes,
      now,
      now
    ],
  )
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
      "#,
    )
    .map_err(|e| format!("DB init failed: {e}"))?;
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
      "SELECT id, company, title, url, raw_text, status, deadline, tags, detected_language, notes, pdf_path, created_at, updated_at
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
        tags: row.get(7)?,
        detected_language: row.get(8)?,
        notes: row.get(9)?,
        pdf_path: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
      })
    })
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;
  Ok(jobs)
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
