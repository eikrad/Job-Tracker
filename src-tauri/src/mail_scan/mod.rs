//! Mail-scan orchestration: sidecar spawn, NDJSON protocol, persistence.
//!
//! Behind `mailScanEnabled` (default off). Scoring/enrichment are stubbed until PR C.

pub mod cluster;
pub mod fingerprint;
pub mod persist;
pub mod protocol;
pub mod scoring;
pub mod spawn;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db;
use crate::secrets::redact;

use self::persist::{
    finish_run, persist_listing, start_run, update_run_stats, upsert_source_cursor, PersistOutcome,
    RunStats,
};
use self::protocol::{read_event_line, Event, ProtocolError, MAX_LINE_BYTES};
use self::scoring::StubScorer;
use self::spawn::{spawn_scan, SpawnedScan};

/// Dev-only gate. Default off so PR B stays invisible.
#[derive(Clone)]
pub struct MailScanFlag(pub Arc<AtomicBool>);

impl Default for MailScanFlag {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}

#[derive(Clone, Default)]
pub struct MailScanRuntime {
    inner: Arc<Mutex<Option<ActiveRun>>>,
}

struct ActiveRun {
    cancel_file: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailScanProgress {
    pub run_id: String,
    pub listings_committed: u32,
    pub messages_seen: u32,
    pub status: String,
}

fn new_run_id() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let suffix: String = (0..8).map(|_| format!("{:02x}", rng.random::<u8>())).collect();
    format!("ms_{}_{}", chrono::Utc::now().timestamp_millis(), suffix)
}

fn python_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../python")
}

fn sidecar_python() -> PathBuf {
    std::env::var_os("JOBTRACKER_MAIL_SCAN_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("python3"))
}

#[tauri::command]
pub fn mail_scan_get_enabled(flag: State<'_, MailScanFlag>) -> bool {
    flag.0.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn mail_scan_set_enabled(enabled: bool, flag: State<'_, MailScanFlag>) -> Result<(), String> {
    flag.0.store(enabled, Ordering::Relaxed);
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanRequest {
    pub sources: Vec<SourceConfig>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    pub id: String,
    pub kind: String,
    pub path: String,
}

#[tauri::command]
pub fn mail_scan_start(
    app: AppHandle,
    flag: State<'_, MailScanFlag>,
    runtime: State<'_, MailScanRuntime>,
    request: StartScanRequest,
) -> Result<String, String> {
    if !flag.0.load(Ordering::Relaxed) {
        return Err("Mail scan is disabled (enable mailScanEnabled in Settings).".into());
    }
    {
        let guard = runtime.inner.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("A mail scan is already running.".into());
        }
    }

    let run_id = new_run_id();
    let cancel_dir = std::env::temp_dir().join("jobtracker-mail-scan");
    std::fs::create_dir_all(&cancel_dir).map_err(|e| e.to_string())?;
    let cancel_file = cancel_dir.join(format!("{run_id}.cancel"));
    let _ = std::fs::remove_file(&cancel_file);

    let mut conn = db::connection(&app)?;
    start_run(&mut conn, &run_id)?;

    let config = serde_json::json!({
        "protocol": 1,
        "run_id": run_id,
        "sources": request.sources.iter().map(|s| serde_json::json!({
            "id": s.id,
            "kind": s.kind,
            "path": s.path,
            "cursor": null,
        })).collect::<Vec<_>>(),
        "extractors": ["indeed", "generic"],
        "limits": {
            "max_messages_per_source": 5000,
            "max_message_bytes": 2_097_152,
            "max_listings_per_run": 2000,
            "max_body_chars": 20_000,
        },
        "since": null,
        "cancel_file": cancel_file.to_string_lossy(),
    });

    let mut child = match spawn_scan(&sidecar_python(), &python_root(), &config.to_string()) {
        Ok(c) => c,
        Err(e) => {
            let mut c = db::connection(&app)?;
            let _ = finish_run(&mut c, &run_id, "failed", Some("E_SPAWN"), Some(&redact(&e)));
            return Err(e);
        }
    };

    {
        let mut guard = runtime.inner.lock().map_err(|e| e.to_string())?;
        *guard = Some(ActiveRun {
            cancel_file: cancel_file.clone(),
        });
    }

    let app2 = app.clone();
    let runtime2 = runtime.inner.clone();
    let run_id2 = run_id.clone();
    std::thread::spawn(move || {
        let result = drive_scan(&app2, &run_id2, &mut child, &cancel_file);
        let mut guard = runtime2.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
        if let Err(e) = result {
            log::warn!("mail scan ended with error: {}", redact(&e));
        }
    });

    Ok(run_id)
}

#[tauri::command]
pub fn mail_scan_cancel(
    flag: State<'_, MailScanFlag>,
    runtime: State<'_, MailScanRuntime>,
) -> Result<(), String> {
    if !flag.0.load(Ordering::Relaxed) {
        return Err("Mail scan is disabled.".into());
    }
    let guard = runtime.inner.lock().map_err(|e| e.to_string())?;
    let Some(active) = guard.as_ref() else {
        return Ok(());
    };
    let _ = std::fs::write(&active.cancel_file, b"1");
    Ok(())
}

fn drive_scan(
    app: &AppHandle,
    run_id: &str,
    child: &mut SpawnedScan,
    _cancel_file: &Path,
) -> Result<(), String> {
    let mut conn = db::connection(app)?;
    let scorer = StubScorer;
    let mut stats = RunStats::default();
    let mut last_emit = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(1))
        .unwrap_or_else(std::time::Instant::now);
    let mut status = "completed".to_string();
    let mut error_code: Option<String> = None;
    let mut error_summary: Option<String> = None;

    let stdout = child
        .stdout
        .as_mut()
        .ok_or_else(|| "sidecar stdout missing".to_string())?;

    let mut line_buf: Vec<u8> = Vec::with_capacity(4096);
    loop {
        match read_event_line(stdout, &mut line_buf, MAX_LINE_BYTES) {
            Ok(None) => break,
            Ok(Some(Event::Unknown { t })) => {
                log::info!("mail scan: skipping unknown event t={t}");
            }
            Ok(Some(Event::Started(ev))) => {
                if ev.protocol != 1 {
                    status = "failed".into();
                    error_code = Some("E_PROTOCOL_MISMATCH".into());
                    error_summary = Some(format!(
                        "protocol mismatch: sidecar={}, expected=1",
                        ev.protocol
                    ));
                    break;
                }
            }
            Ok(Some(Event::Listing(listing))) => {
                match persist_listing(&mut conn, run_id, listing.as_ref(), &scorer) {
                    Ok(PersistOutcome::Committed) => {
                        stats.listings_committed += 1;
                    }
                    Ok(PersistOutcome::SuppressedByDismissal) => {
                        stats.suppressed_by_dismissal += 1;
                    }
                    Err(e) => {
                        status = "failed".into();
                        error_code = Some("E_PERSIST".into());
                        error_summary = Some(redact(&e));
                        break;
                    }
                }
            }
            Ok(Some(Event::SourceFinished(ev))) => {
                let _ = upsert_source_cursor(
                    &mut conn,
                    &ev.source,
                    "",
                    "mbox",
                    ev.cursor.size as i64,
                    ev.cursor.mtime_ns as i64,
                    ev.cursor.offset as i64,
                    ev.cursor.last_message_id.as_deref(),
                    ev.cursor.sentinel_hash.as_deref(),
                );
                stats.messages_parsed = stats.messages_parsed.saturating_add(ev.messages_read);
            }
            Ok(Some(Event::SourceStarted(_))) | Ok(Some(Event::Warning(_))) => {}
            Ok(Some(Event::Finished(ev))) => {
                stats.messages_seen = ev.messages_total;
                if ev.cancelled.unwrap_or(false) {
                    status = "cancelled".into();
                }
            }
            Err(ProtocolError::Oversize) => {
                status = "failed".into();
                error_code = Some("E_PROTOCOL_OVERSIZE".into());
                error_summary = Some("NDJSON line exceeded 256 KiB".into());
                break;
            }
            Err(ProtocolError::Malformed(detail)) => {
                status = "failed".into();
                error_code = Some("E_PROTOCOL".into());
                error_summary = Some(redact(&detail));
                break;
            }
            Err(ProtocolError::Io(e)) => {
                status = "failed".into();
                error_code = Some("E_IO".into());
                error_summary = Some(redact(&e));
                break;
            }
        }

        if last_emit.elapsed() >= std::time::Duration::from_millis(250) {
            let _ = update_run_stats(&mut conn, run_id, &stats);
            let _ = app.emit(
                "mail-scan://progress",
                MailScanProgress {
                    run_id: run_id.to_string(),
                    listings_committed: stats.listings_committed,
                    messages_seen: stats.messages_seen,
                    status: "running".into(),
                },
            );
            last_emit = std::time::Instant::now();
        }
    }

    let exit = child.wait();
    match exit {
        Ok(Some(10)) => {
            if status == "completed" {
                status = "cancelled".into();
            }
        }
        Ok(Some(0)) => {}
        Ok(Some(code)) if status == "completed" => {
            status = "failed".into();
            error_code = error_code.or(Some(format!("E_EXIT_{code}")));
        }
        Ok(None) | Err(_) => {
            if status == "completed" {
                status = "failed".into();
                error_code = error_code.or(Some("E_CHILD".into()));
            }
        }
        Ok(Some(_)) => {}
    }

    let _ = update_run_stats(&mut conn, run_id, &stats);
    finish_run(
        &mut conn,
        run_id,
        &status,
        error_code.as_deref(),
        error_summary.as_deref(),
    )?;
    let _ = app.emit(
        "mail-scan://progress",
        MailScanProgress {
            run_id: run_id.to_string(),
            listings_committed: stats.listings_committed,
            messages_seen: stats.messages_seen,
            status: status.clone(),
        },
    );
    Ok(())
}

/// Process an in-memory NDJSON stream (tests / crash-safety harness).
#[cfg(test)]
pub fn consume_event_stream<R: std::io::Read>(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    reader: R,
    scorer: &dyn scoring::ListingScorer,
) -> Result<RunStats, String> {
    use std::io::BufRead;
    start_run(conn, run_id)?;
    let mut stats = RunStats::default();
    let mut status = "completed".to_string();
    let mut error_code: Option<String> = None;
    let mut error_summary: Option<String> = None;

    let mut buffered = std::io::BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        let n = buffered.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        if line.len() > MAX_LINE_BYTES {
            status = "failed".into();
            error_code = Some("E_PROTOCOL_OVERSIZE".into());
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            continue;
        }
        match protocol::parse_event_line(trimmed) {
            Ok(Event::Listing(listing)) => {
                match persist_listing(conn, run_id, listing.as_ref(), scorer)? {
                    PersistOutcome::Committed => stats.listings_committed += 1,
                    PersistOutcome::SuppressedByDismissal => stats.suppressed_by_dismissal += 1,
                }
            }
            Ok(Event::SourceFinished(ev)) => {
                stats.messages_parsed =
                    stats.messages_parsed.saturating_add(ev.messages_read);
            }
            Ok(Event::Finished(ev)) => {
                stats.messages_seen = ev.messages_total;
                if ev.cancelled.unwrap_or(false) {
                    status = "cancelled".into();
                }
            }
            Ok(Event::Unknown { .. }) | Ok(_) => {}
            Err(ProtocolError::Malformed(detail)) => {
                status = "failed".into();
                error_code = Some("E_PROTOCOL".into());
                error_summary = Some(detail);
                break;
            }
            Err(e) => {
                status = "failed".into();
                error_code = Some("E_PROTOCOL".into());
                error_summary = Some(e.to_string());
                break;
            }
        }
    }

    update_run_stats(conn, run_id, &stats)?;
    finish_run(
        conn,
        run_id,
        &status,
        error_code.as_deref(),
        error_summary.as_deref(),
    )?;
    Ok(stats)
}
