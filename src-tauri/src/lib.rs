use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Serialize, Deserialize)]
struct Job {
  id: i64,
  company: String,
  title: Option<String>,
  url: Option<String>,
  raw_text: Option<String>,
  status: String,
  deadline: Option<String>,
  tags: Option<String>,
  detected_language: Option<String>,
  notes: Option<String>,
  pdf_path: Option<String>,
  created_at: String,
  updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct NewJob {
  company: String,
  title: Option<String>,
  url: Option<String>,
  raw_text: Option<String>,
  status: String,
  deadline: Option<String>,
  tags: Option<String>,
  detected_language: Option<String>,
  notes: Option<String>,
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

fn connection(app: &tauri::AppHandle) -> Result<Connection, String> {
  let path = db_path(app)?;
  Connection::open(path).map_err(|e| format!("DB open failed: {e}"))
}

#[tauri::command]
fn init_db(app: tauri::AppHandle) -> Result<(), String> {
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
fn create_job(app: tauri::AppHandle, payload: NewJob) -> Result<i64, String> {
  let conn = connection(&app)?;
  let now = Utc::now().to_rfc3339();
  conn
    .execute(
      "INSERT INTO jobs (company, title, url, raw_text, status, deadline, tags, detected_language, notes, created_at, updated_at)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
      params![
        payload.company,
        payload.title,
        payload.url,
        payload.raw_text,
        payload.status,
        payload.deadline,
        payload.tags,
        payload.detected_language,
        payload.notes,
        now,
        now
      ],
    )
    .map_err(|e| format!("Create job failed: {e}"))?;
  Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn list_jobs(app: tauri::AppHandle) -> Result<Vec<Job>, String> {
  let conn = connection(&app)?;
  let mut stmt = conn
    .prepare(
      "SELECT id, company, title, url, raw_text, status, deadline, tags, detected_language, notes, pdf_path, created_at, updated_at
      FROM jobs ORDER BY updated_at DESC",
    )
    .map_err(|e| e.to_string())?;
  let rows = stmt
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
    .map_err(|e| e.to_string())?;

  let mut jobs = Vec::new();
  for row in rows {
    jobs.push(row.map_err(|e| e.to_string())?);
  }
  Ok(jobs)
}

#[tauri::command]
fn update_job_status(app: tauri::AppHandle, job_id: i64, new_status: String) -> Result<(), String> {
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
fn list_status_history(app: tauri::AppHandle, job_id: i64) -> Result<Vec<serde_json::Value>, String> {
  let conn = connection(&app)?;
  let mut stmt = conn
    .prepare("SELECT from_status, to_status, changed_at FROM status_history WHERE job_id = ?1 ORDER BY changed_at DESC")
    .map_err(|e| e.to_string())?;
  let rows = stmt
    .query_map(params![job_id], |row| {
      Ok(serde_json::json!({
        "from_status": row.get::<_, Option<String>>(0)?,
        "to_status": row.get::<_, String>(1)?,
        "changed_at": row.get::<_, String>(2)?,
      }))
    })
    .map_err(|e| e.to_string())?;
  let mut out = Vec::new();
  for row in rows {
    out.push(row.map_err(|e| e.to_string())?);
  }
  Ok(out)
}

#[tauri::command]
fn save_application_pdf(
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      init_db,
      create_job,
      list_jobs,
      update_job_status,
      list_status_history,
      save_application_pdf
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
