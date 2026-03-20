use chrono::{Duration, NaiveDate, Utc};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleCalendarCreateEventArgs {
  access_token: String,
  job_id: i64,
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

fn deadline_end_exclusive(deadline: &str) -> Result<String, String> {
  let d = NaiveDate::parse_from_str(deadline, "%Y-%m-%d").map_err(|e| e.to_string())?;
  Ok((d + Duration::days(1)).format("%Y-%m-%d").to_string())
}

#[tauri::command]
fn import_jobs(app: tauri::AppHandle, jobs: Vec<NewJob>) -> Result<usize, String> {
  let conn = connection(&app)?;
  let now = Utc::now().to_rfc3339();
  let mut count = 0usize;
  for payload in jobs {
    if payload.company.trim().is_empty() {
      continue;
    }
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
      .map_err(|e| format!("Import job failed: {e}"))?;
    count += 1;
  }
  Ok(count)
}

#[tauri::command]
fn google_calendar_create_event(
  app: tauri::AppHandle,
  args: GoogleCalendarCreateEventArgs,
) -> Result<String, String> {
  let conn = connection(&app)?;
  let row: (String, Option<String>, Option<String>) = conn
    .query_row(
      "SELECT company, title, deadline FROM jobs WHERE id = ?1",
      params![args.job_id],
      |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .map_err(|e| format!("Job lookup failed: {e}"))?;
  let (company, title, deadline) = row;
  let deadline = deadline.ok_or_else(|| "Job has no deadline".to_string())?;
  let end_date = deadline_end_exclusive(&deadline)?;
  let summary = format!(
    "Application deadline: {} — {}",
    company,
    title.unwrap_or_else(|| "Role".to_string())
  );
  let body = serde_json::json!({
    "summary": summary,
    "description": format!("Synced from Job Tracker (job id {}).", args.job_id),
    "start": { "date": deadline },
    "end": { "date": end_date },
  });
  let client = reqwest::blocking::Client::new();
  let res = client
    .post("https://www.googleapis.com/calendar/v3/calendars/primary/events")
    .header("Authorization", format!("Bearer {}", args.access_token.trim()))
    .header("Content-Type", "application/json")
    .json(&body)
    .send()
    .map_err(|e| format!("HTTP request failed: {e}"))?;
  if !res.status().is_success() {
    let text = res.text().unwrap_or_default();
    return Err(format!("Google Calendar API error: {text}"));
  }
  let json: serde_json::Value = res.json().map_err(|e| e.to_string())?;
  let link = json
    .get("htmlLink")
    .and_then(|v| v.as_str())
    .unwrap_or("Event created")
    .to_string();
  Ok(link)
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
      save_application_pdf,
      import_jobs,
      google_calendar_create_event
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
