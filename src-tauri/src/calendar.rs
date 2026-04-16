use chrono::{Duration, NaiveDate};
use rusqlite::params;
use serde::Deserialize;
use tauri::AppHandle;

use crate::db::connection;
use crate::google_oauth;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCalendarCreateEventArgs {
    pub job_id: i64,
    /// `apply` | `interview` | `start`
    pub date_kind: String,
    /// Legacy: paste token in Settings (Advanced). If empty/missing, uses OAuth refresh token.
    pub access_token: Option<String>,
}

pub(crate) fn deadline_end_exclusive(deadline: &str) -> Result<String, String> {
    let d = NaiveDate::parse_from_str(deadline, "%Y-%m-%d").map_err(|e| e.to_string())?;
    Ok((d + Duration::days(1)).format("%Y-%m-%d").to_string())
}

fn pick_date_and_summary(
    company: &str,
    title: Option<String>,
    job_id: i64,
    kind: &str,
    deadline: Option<String>,
    interview_date: Option<String>,
    start_date: Option<String>,
) -> Result<(String, String, String), String> {
    let role = title.unwrap_or_else(|| "Role".to_string());
    let (date, summary) = match kind {
        "apply" => {
            let d = deadline.ok_or_else(|| "This job has no application deadline.".to_string())?;
            (d, format!("Application deadline: {company} — {role}"))
        }
        "interview" => {
            let d = interview_date.ok_or_else(|| "This job has no interview date.".to_string())?;
            (d, format!("Interview: {company} — {role}"))
        }
        "start" => {
            let d = start_date.ok_or_else(|| "This job has no start date.".to_string())?;
            (d, format!("Role start: {company} — {role}"))
        }
        _ => return Err(format!("Unknown date_kind: {kind}")),
    };
    let description = format!("Synced from Job Tracker (job id {job_id}, {kind}).");
    Ok((date, summary, description))
}

#[tauri::command]
pub fn google_calendar_create_event(
    app: AppHandle,
    args: GoogleCalendarCreateEventArgs,
) -> Result<String, String> {
    let token_override = args
        .access_token
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let access_token = google_oauth::resolve_calendar_access_token(&app, token_override)?;

    let conn = connection(&app)?;
    let row: (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT company, title, deadline, interview_date, start_date FROM jobs WHERE id = ?1",
            params![args.job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|e| format!("Job lookup failed: {e}"))?;
    let (company, title, deadline, interview_date, start_date) = row;
    let kind = args.date_kind.trim();
    let (start_d, summary, description) = pick_date_and_summary(
        &company,
        title,
        args.job_id,
        kind,
        deadline,
        interview_date,
        start_date,
    )?;
    let end_date = deadline_end_exclusive(&start_d)?;
    let body = serde_json::json!({
      "summary": summary,
      "description": description,
      "start": { "date": start_d },
      "end": { "date": end_date },
    });
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://www.googleapis.com/calendar/v3/calendars/primary/events")
        .header("Authorization", format!("Bearer {}", access_token.trim()))
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
    use super::{deadline_end_exclusive, pick_date_and_summary, GoogleCalendarCreateEventArgs};

    #[test]
    fn deadline_end_exclusive_adds_one_day() {
        assert_eq!(deadline_end_exclusive("2026-04-01").unwrap(), "2026-04-02");
    }

    #[test]
    fn deadline_end_exclusive_crosses_year() {
        assert_eq!(deadline_end_exclusive("2026-12-31").unwrap(), "2027-01-01");
    }

    #[test]
    fn deadline_end_exclusive_rejects_bad_input() {
        assert!(deadline_end_exclusive("32-13-99").is_err());
        assert!(deadline_end_exclusive("not-a-date").is_err());
    }

    #[test]
    fn google_calendar_args_deserializes_camel_case_json() {
        let json = r#"{"accessToken":"secret-token","jobId":42,"dateKind":"apply"}"#;
        let args: GoogleCalendarCreateEventArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.access_token.as_deref(), Some("secret-token"));
        assert_eq!(args.job_id, 42);
        assert_eq!(args.date_kind, "apply");
    }

    #[test]
    fn pick_date_apply_interview_start() {
        let (d, s, _) = pick_date_and_summary(
            "Acme",
            Some("Dev".into()),
            1,
            "apply",
            Some("2026-06-01".into()),
            None,
            None,
        )
        .unwrap();
        assert_eq!(d, "2026-06-01");
        assert!(s.contains("Application deadline"));

        let (d2, s2, _) = pick_date_and_summary(
            "Acme",
            None,
            1,
            "interview",
            None,
            Some("2026-06-15".into()),
            None,
        )
        .unwrap();
        assert_eq!(d2, "2026-06-15");
        assert!(s2.contains("Interview"));

        let (d3, s3, _) = pick_date_and_summary(
            "Acme",
            None,
            1,
            "start",
            None,
            None,
            Some("2026-07-01".into()),
        )
        .unwrap();
        assert_eq!(d3, "2026-07-01");
        assert!(s3.contains("Role start"));
    }
}
