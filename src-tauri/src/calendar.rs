use chrono::{Duration, NaiveDate};
use rusqlite::params;
use serde::Deserialize;
use tauri::AppHandle;

use crate::db::connection;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCalendarCreateEventArgs {
  pub access_token: String,
  pub job_id: i64,
}

pub(crate) fn deadline_end_exclusive(deadline: &str) -> Result<String, String> {
  let d = NaiveDate::parse_from_str(deadline, "%Y-%m-%d").map_err(|e| e.to_string())?;
  Ok((d + Duration::days(1)).format("%Y-%m-%d").to_string())
}

#[tauri::command]
pub fn google_calendar_create_event(
  app: AppHandle,
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

#[cfg(test)]
mod tests {
  use super::{deadline_end_exclusive, GoogleCalendarCreateEventArgs};

  #[test]
  fn deadline_end_exclusive_adds_one_day() {
    assert_eq!(
      deadline_end_exclusive("2026-04-01").unwrap(),
      "2026-04-02"
    );
  }

  #[test]
  fn deadline_end_exclusive_crosses_year() {
    assert_eq!(
      deadline_end_exclusive("2026-12-31").unwrap(),
      "2027-01-01"
    );
  }

  #[test]
  fn deadline_end_exclusive_rejects_bad_input() {
    assert!(deadline_end_exclusive("32-13-99").is_err());
    assert!(deadline_end_exclusive("not-a-date").is_err());
  }

  #[test]
  fn google_calendar_args_deserializes_camel_case_json() {
    let json = r#"{"accessToken":"secret-token","jobId":42}"#;
    let args: GoogleCalendarCreateEventArgs = serde_json::from_str(json).unwrap();
    assert_eq!(args.access_token, "secret-token");
    assert_eq!(args.job_id, 42);
  }
}
