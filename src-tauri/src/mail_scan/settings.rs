//! Mail-scan configuration and destructive data actions (spec §6.4, §6.5, §11.3).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mail_scan::budget::DEFAULT_MAX_CALLS;
use crate::mail_scan::scoring::DEFAULT_PASS2_CUTOFF;
use crate::mail_scan::DEFAULT_SINCE_DAYS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MailSource {
    pub id: String,
    pub label: String,
    /// As the user entered it — may be relative, contain `~`, or be a symlink.
    pub path: String,
    /// `mbox` | `maildir`, detected from the path.
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailScanSettings {
    #[serde(default)]
    pub sources: Vec<MailSource>,
    #[serde(default = "default_cutoff")]
    pub cutoff: i32,
    #[serde(default = "default_since_days")]
    pub since_days: u32,
    #[serde(default = "default_max_calls")]
    pub max_calls: u32,
    #[serde(default = "default_provider")]
    pub provider: String,
}

fn default_cutoff() -> i32 {
    DEFAULT_PASS2_CUTOFF
}
fn default_since_days() -> u32 {
    DEFAULT_SINCE_DAYS
}
fn default_max_calls() -> u32 {
    DEFAULT_MAX_CALLS
}
fn default_provider() -> String {
    "scaleway_deepseek".into()
}

impl Default for MailScanSettings {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            cutoff: default_cutoff(),
            since_days: default_since_days(),
            max_calls: default_max_calls(),
            provider: default_provider(),
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("mail_scan_settings.json"))
}

pub fn load_settings(app: &tauri::AppHandle) -> Result<MailScanSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(MailScanSettings::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        return Ok(MailScanSettings::default());
    }
    serde_json::from_str(&raw).map_err(|e| format!("Invalid mail_scan_settings.json: {e}"))
}

pub fn save_settings(app: &tauri::AppHandle, settings: &MailScanSettings) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(app)?, raw).map_err(|e| e.to_string())
}

/// A configured folder, resolved.
///
/// The **resolved** path is what Settings displays, so a symlinked "Indeed" folder
/// pointing at `~/.ssh` is visible before it is ever read (spec §6.4).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSource {
    pub id: String,
    pub label: String,
    pub entered_path: String,
    pub resolved_path: Option<String>,
    pub kind: String,
    pub exists: bool,
    /// True when the entered path resolved somewhere else — a symlink or `..`.
    pub redirected: bool,
    pub error: Option<String>,
}

/// mbox is a regular file; maildir is a directory containing `cur`/`new`.
fn detect_kind(resolved: &Path) -> &'static str {
    if resolved.is_dir() {
        "maildir"
    } else {
        "mbox"
    }
}

pub fn resolve_source(source: &MailSource) -> ResolvedSource {
    let expanded = PathBuf::from(shellexpand::tilde(&source.path).as_ref());
    let mut out = ResolvedSource {
        id: source.id.clone(),
        label: source.label.clone(),
        entered_path: source.path.clone(),
        resolved_path: None,
        kind: source.kind.clone(),
        exists: false,
        redirected: false,
        error: None,
    };

    match expanded.canonicalize() {
        Ok(resolved) => {
            if !resolved.is_file() && !resolved.is_dir() {
                out.error = Some("E_SOURCE_UNREADABLE: not a file or a folder.".into());
                return out;
            }
            out.exists = true;
            out.redirected = resolved != expanded;
            out.kind = detect_kind(&resolved).to_string();
            out.resolved_path = Some(resolved.to_string_lossy().into_owned());
        }
        Err(e) => {
            out.error = Some(format!("E_SOURCE_UNREADABLE: {e}"));
        }
    }
    out
}

/// What `Test read` reports (spec §11.3).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceTestResult {
    pub id: String,
    pub ok: bool,
    pub message_count: u64,
    pub earliest: Option<String>,
    pub latest: Option<String>,
    pub resolved_path: Option<String>,
    pub error: Option<String>,
}

/// Count messages and date range without parsing bodies.
///
/// Deliberately cheap: for mbox this counts `From ` separators and reads `Date:`
/// headers, so pointing at a 200 MB folder to check a path does not read 200 MB of
/// bodies.
pub fn test_read(source: &MailSource) -> SourceTestResult {
    let resolved = resolve_source(source);
    let mut result = SourceTestResult {
        id: source.id.clone(),
        ok: false,
        message_count: 0,
        earliest: None,
        latest: None,
        resolved_path: resolved.resolved_path.clone(),
        error: resolved.error.clone(),
    };
    let Some(path) = resolved.resolved_path.as_ref().map(PathBuf::from) else {
        return result;
    };

    if path.is_dir() {
        let mut count = 0u64;
        for sub in ["cur", "new"] {
            if let Ok(entries) = std::fs::read_dir(path.join(sub)) {
                count += entries.flatten().filter(|e| e.path().is_file()).count() as u64;
            }
        }
        result.ok = true;
        result.message_count = count;
        return result;
    }

    match count_mbox(&path) {
        Ok((count, earliest, latest)) => {
            result.ok = true;
            result.message_count = count;
            result.earliest = earliest;
            result.latest = latest;
        }
        Err(e) => result.error = Some(format!("E_SOURCE_UNREADABLE: {e}")),
    }
    result
}

type MboxSummary = (u64, Option<String>, Option<String>);

fn count_mbox(path: &Path) -> Result<MboxSummary, String> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    let mut count = 0u64;
    let mut earliest: Option<String> = None;
    let mut latest: Option<String> = None;
    let mut at_message_start = false;

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.starts_with("From ") {
            count += 1;
            at_message_start = true;
            continue;
        }
        // Only the first Date: of each message, so a quoted reply cannot skew the range.
        if at_message_start && line.starts_with("Date:") {
            let value = line.trim_start_matches("Date:").trim().to_string();
            if earliest.is_none() {
                earliest = Some(value.clone());
            }
            latest = Some(value);
            at_message_start = false;
        } else if line.is_empty() {
            at_message_start = false;
        }
    }
    Ok((count, earliest, latest))
}

/// Best-effort Thunderbird mail directories (spec §13: Linux first, manual always works).
pub fn detect_thunderbird_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let Some(home) = home else { return roots };

    // Linux and macOS layouts; Windows uses %APPDATA% and is left to manual picking.
    for relative in [
        ".thunderbird",
        ".mozilla-thunderbird",
        "Library/Thunderbird/Profiles",
        "snap/thunderbird/common/.thunderbird",
        ".var/app/org.mozilla.Thunderbird/.thunderbird",
    ] {
        let candidate = home.join(relative);
        if candidate.is_dir() {
            roots.push(candidate);
        }
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let candidate = PathBuf::from(appdata).join("Thunderbird").join("Profiles");
        if candidate.is_dir() {
            roots.push(candidate);
        }
    }
    roots
}

/// Tables `Delete all mail scan data` clears, in FK-safe order.
///
/// `jobs` is absent and must stay absent: the point of the action is to forget the
/// scanning, not the applications it produced (spec §6.5).
const MAIL_SCAN_TABLES: &[&str] = &[
    "mail_scored_sightings",
    "mail_match_dismissals",
    "mail_match_inbox",
    "mail_fingerprint_aliases",
    "mail_fingerprints",
    "mail_source_cursors",
    "mail_scan_runs",
];

/// Drop every trace of scanning, leaving Jobs intact.
pub fn delete_all_mail_scan_data(conn: &mut rusqlite::Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for table in MAIL_SCAN_TABLES {
        tx.execute(&format!("DELETE FROM {table}"), [])
            .map_err(|e| format!("clearing {table}: {e}"))?;
    }
    // Accepted matches leave provenance rows behind; those describe Jobs, so the
    // rows go but the Jobs and their field values stay.
    tx.execute("DELETE FROM job_field_provenance WHERE source = 'mail_scan'", [])
        .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE jobs SET mail_score = NULL, mail_score_reason = NULL, mail_scored_at = NULL",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn mail_scan_settings_get(app: tauri::AppHandle) -> Result<MailScanSettings, String> {
    load_settings(&app)
}

#[tauri::command]
pub fn mail_scan_settings_set(
    app: tauri::AppHandle,
    settings: MailScanSettings,
) -> Result<(), String> {
    save_settings(&app, &settings)
}

#[tauri::command]
pub fn mail_scan_resolve_sources(app: tauri::AppHandle) -> Result<Vec<ResolvedSource>, String> {
    Ok(load_settings(&app)?.sources.iter().map(resolve_source).collect())
}

#[tauri::command]
pub fn mail_scan_test_source(source: MailSource) -> SourceTestResult {
    test_read(&source)
}

#[tauri::command]
pub fn mail_scan_detect_thunderbird() -> Vec<String> {
    detect_thunderbird_roots()
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[tauri::command]
pub fn mail_scan_sidecar_probe(app: tauri::AppHandle) -> crate::mail_scan::sidecar::SidecarProbe {
    let _ = &app;
    crate::mail_scan::sidecar::probe(
        &crate::mail_scan::python_root(),
        std::env::var_os("JOBTRACKER_MAIL_SCAN_PYTHON").map(PathBuf::from),
    )
}

/// Destructive; the UI gates this behind a typed confirmation (spec §6.5).
#[tauri::command]
pub fn mail_scan_delete_all_data(app: tauri::AppHandle, confirmation: String) -> Result<(), String> {
    if confirmation.trim() != "DELETE" {
        return Err("Type DELETE to confirm.".into());
    }
    let mut conn = crate::db::connection(&app)?;
    delete_all_mail_scan_data(&mut conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use std::fs;

    fn source(path: &str) -> MailSource {
        MailSource {
            id: "s1".into(),
            label: "Indeed".into(),
            path: path.into(),
            kind: "mbox".into(),
        }
    }

    #[test]
    fn a_symlinked_folder_shows_where_it_actually_points() {
        // The §6.4 case: an "Indeed" folder pointing at something else must be
        // visible in Settings before it is read.
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("secrets");
        fs::create_dir_all(&real).unwrap();
        let link = dir.path().join("Indeed");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&real, &link).unwrap();
        #[cfg(not(unix))]
        return;

        let resolved = resolve_source(&source(link.to_str().unwrap()));

        assert!(resolved.exists);
        assert!(resolved.redirected, "a symlink must be flagged as redirected");
        assert!(resolved.resolved_path.unwrap().contains("secrets"));
    }

    #[test]
    fn a_missing_folder_reports_the_taxonomy_code() {
        let resolved = resolve_source(&source("/definitely/not/here.mbox"));
        assert!(!resolved.exists);
        assert!(resolved.error.unwrap().starts_with("E_SOURCE_UNREADABLE"));
    }

    #[test]
    fn kind_is_detected_rather_than_trusted() {
        let dir = tempfile::tempdir().unwrap();
        let mbox = dir.path().join("inbox.mbox");
        fs::write(&mbox, "From a@b Mon Sep 08 2026\n").unwrap();

        // The caller claimed maildir; the filesystem says otherwise.
        let mut s = source(mbox.to_str().unwrap());
        s.kind = "maildir".into();
        assert_eq!(resolve_source(&s).kind, "mbox");

        let maildir = dir.path().join("Mail");
        fs::create_dir_all(maildir.join("cur")).unwrap();
        assert_eq!(resolve_source(&source(maildir.to_str().unwrap())).kind, "maildir");
    }

    #[test]
    fn test_read_reports_count_and_date_range() {
        let dir = tempfile::tempdir().unwrap();
        let mbox = dir.path().join("inbox.mbox");
        fs::write(
            &mbox,
            "From a@b Mon Sep 01 2026\nDate: Mon, 01 Sep 2026 08:00:00 +0000\nSubject: one\n\nbody\n\
             From a@b Tue Sep 09 2026\nDate: Tue, 09 Sep 2026 08:00:00 +0000\nSubject: two\n\nbody\n",
        )
        .unwrap();

        let result = test_read(&source(mbox.to_str().unwrap()));

        assert!(result.ok);
        assert_eq!(result.message_count, 2);
        assert!(result.earliest.unwrap().contains("01 Sep"));
        assert!(result.latest.unwrap().contains("09 Sep"));
    }

    #[test]
    fn test_read_counts_maildir_entries() {
        let dir = tempfile::tempdir().unwrap();
        let maildir = dir.path().join("Mail");
        fs::create_dir_all(maildir.join("cur")).unwrap();
        fs::create_dir_all(maildir.join("new")).unwrap();
        fs::write(maildir.join("cur/1"), "x").unwrap();
        fs::write(maildir.join("new/2"), "x").unwrap();

        let result = test_read(&source(maildir.to_str().unwrap()));
        assert!(result.ok);
        assert_eq!(result.message_count, 2);
    }

    #[test]
    fn test_read_on_a_missing_path_fails_without_panicking() {
        let result = test_read(&source("/definitely/not/here.mbox"));
        assert!(!result.ok);
        assert!(result.error.is_some());
    }

    #[test]
    fn deleting_mail_scan_data_leaves_jobs_intact() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        migrations::apply_connection_pragmas(&conn).unwrap();
        migrations::run(&mut conn).unwrap();

        conn.execute(
            "INSERT INTO jobs (company, title, status, deadline, priority, mail_score,
                               mail_score_reason, created_at, updated_at)
             VALUES ('Acme','Rust Engineer','Applied','2026-10-01',2,9,'good','t','t')",
            [],
        )
        .unwrap();
        let job_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO mail_scan_runs (run_id, status, started_at) VALUES ('r1','completed','t')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mail_fingerprints (fingerprint_id, strong_key, weak_key, first_seen_at, last_seen_at)
             VALUES ('fp1','s','w','t','t')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO mail_match_inbox (fingerprint_id, kind, status, score_state, draft_json,
                                           enrichment_state, first_run_id, last_run_id, created_at, updated_at)
             VALUES ('fp1','new','pending','ok','{}','complete','r1','r1','t','t')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO job_field_provenance (job_id, field, source, run_id, set_at)
             VALUES (?1, 'deadline', 'mail_scan', 'r1', 't')",
            rusqlite::params![job_id],
        )
        .unwrap();

        delete_all_mail_scan_data(&mut conn).unwrap();

        for table in MAIL_SCAN_TABLES {
            let n: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(n, 0, "{table} should be empty");
        }

        // The Job, and the fields the scan filled in, survive.
        let (company, deadline, priority, status, score): (
            String,
            Option<String>,
            Option<i64>,
            String,
            Option<i64>,
        ) = conn
            .query_row(
                "SELECT company, deadline, priority, status, mail_score FROM jobs WHERE id = ?1",
                rusqlite::params![job_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(company, "Acme");
        assert_eq!(deadline.as_deref(), Some("2026-10-01"));
        assert_eq!(priority, Some(2));
        assert_eq!(status, "Applied");
        assert_eq!(score, None, "the advisory score goes with the scan data");
    }

    #[test]
    fn the_delete_list_never_names_the_jobs_table() {
        assert!(!MAIL_SCAN_TABLES.contains(&"jobs"));
        assert!(!MAIL_SCAN_TABLES.contains(&"job_documents"));
        assert!(!MAIL_SCAN_TABLES.contains(&"job_status_history"));
    }

    #[test]
    fn settings_defaults_match_the_spec() {
        let defaults = MailScanSettings::default();
        assert_eq!(defaults.cutoff, 7);
        assert_eq!(defaults.since_days, 90);
        assert_eq!(defaults.max_calls, 600);
        assert!(defaults.sources.is_empty());
    }

    #[test]
    fn settings_json_tolerates_missing_fields() {
        // An older settings file must not blank the user's folders.
        let parsed: MailScanSettings =
            serde_json::from_str(r#"{"sources":[{"id":"a","label":"L","path":"/x","kind":"mbox"}]}"#)
                .unwrap();
        assert_eq!(parsed.sources.len(), 1);
        assert_eq!(parsed.cutoff, 7);
        assert_eq!(parsed.since_days, 90);
    }
}
