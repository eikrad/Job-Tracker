//! Mail-scan orchestration: sidecar spawn, NDJSON protocol, persistence.
//!
//! Behind `mailScanEnabled` (default off). Scoring/enrichment are stubbed until PR C.

pub mod cluster;
pub mod fingerprint;
pub mod persist;
pub mod protocol;
pub mod scoring;
pub mod spawn;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db;
use crate::secrets::redact;

use self::persist::{
    finish_run, persist_listing, start_run, update_run_stats, upsert_source_cursor, PersistOutcome,
    RunStats,
};
use self::protocol::{read_event_line, Event, ProtocolError, MAX_LINE_BYTES};
use self::scoring::{ListingScorer, StubScorer};
use self::spawn::{spawn_scan, watch_cancel_escalation, SpawnedScan};

/// Extractor names shipped with the sidecar (keep in sync with Python DEFAULT_EXTRACTORS).
pub const DEFAULT_EXTRACTORS: &[&str] = &["indeed", "generic"];

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

struct DriveState {
    stats: RunStats,
    status: String,
    error_code: Option<String>,
    error_summary: Option<String>,
    /// source_id → (path, kind) for cursor commits
    sources: HashMap<String, (String, String)>,
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

fn load_cursor_json(
    conn: &rusqlite::Connection,
    source_id: &str,
) -> Result<Option<serde_json::Value>, String> {
    type CursorRow = (i64, i64, i64, Option<String>, Option<String>);
    let row: Option<CursorRow> = conn
        .query_row(
            "SELECT size, mtime_ns, offset, last_message_id, sentinel_hash
             FROM mail_source_cursors WHERE source_id = ?1",
            rusqlite::params![source_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(row.map(|(size, mtime_ns, offset, last_message_id, sentinel_hash)| {
        serde_json::json!({
            "size": size,
            "mtime_ns": mtime_ns,
            "offset": offset,
            "last_message_id": last_message_id,
            "sentinel_hash": sentinel_hash,
        })
    }))
}

/// Apply one protocol event to DB/stats. Returns false when the run should stop.
fn apply_event(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    event: Event,
    scorer: &dyn ListingScorer,
    state: &mut DriveState,
) -> Result<bool, String> {
    match event {
        Event::Unknown { t } => {
            log::info!("mail scan: skipping unknown event t={t}");
            Ok(true)
        }
        Event::Started(ev) => {
            if ev.protocol != 1 {
                state.status = "failed".into();
                state.error_code = Some("E_PROTOCOL_MISMATCH".into());
                state.error_summary = Some(format!(
                    "protocol mismatch: sidecar={}, expected=1",
                    ev.protocol
                ));
                return Ok(false);
            }
            Ok(true)
        }
        Event::Listing(listing) => match persist_listing(conn, run_id, listing.as_ref(), scorer) {
            Ok(PersistOutcome::Committed) => {
                state.stats.listings_committed += 1;
                Ok(true)
            }
            Ok(PersistOutcome::SuppressedByDismissal) => {
                state.stats.suppressed_by_dismissal += 1;
                Ok(true)
            }
            Err(e) => {
                state.status = "failed".into();
                state.error_code = Some("E_PERSIST".into());
                state.error_summary = Some(redact(&e));
                Ok(false)
            }
        },
        Event::SourceFinished(ev) => {
            let (path, kind) = state
                .sources
                .get(&ev.source)
                .cloned()
                .unwrap_or_else(|| (String::new(), String::from("mbox")));
            let _ = upsert_source_cursor(
                conn,
                &ev.source,
                &path,
                &kind,
                ev.cursor.size as i64,
                ev.cursor.mtime_ns as i64,
                ev.cursor.offset as i64,
                ev.cursor.last_message_id.as_deref(),
                ev.cursor.sentinel_hash.as_deref(),
            );
            state.stats.messages_parsed = state
                .stats
                .messages_parsed
                .saturating_add(ev.messages_read);
            Ok(true)
        }
        Event::SourceStarted(_) | Event::Warning(_) => Ok(true),
        Event::Finished(ev) => {
            state.stats.messages_seen = ev.messages_total;
            if ev.cancelled.unwrap_or(false) {
                state.status = "cancelled".into();
            }
            Ok(true)
        }
    }
}

fn consume_reader<R: std::io::Read>(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    reader: &mut R,
    scorer: &dyn ListingScorer,
    state: &mut DriveState,
    mut on_tick: impl FnMut(&mut rusqlite::Connection, &DriveState),
) -> Result<(), String> {
    let mut line_buf: Vec<u8> = Vec::with_capacity(4096);
    loop {
        match read_event_line(reader, &mut line_buf, MAX_LINE_BYTES) {
            Ok(None) => break,
            Ok(Some(event)) => {
                if !apply_event(conn, run_id, event, scorer, state)? {
                    break;
                }
            }
            Err(ProtocolError::Oversize) => {
                state.status = "failed".into();
                state.error_code = Some("E_PROTOCOL_OVERSIZE".into());
                state.error_summary = Some("NDJSON line exceeded 256 KiB".into());
                break;
            }
            Err(ProtocolError::Malformed(detail)) => {
                state.status = "failed".into();
                state.error_code = Some("E_PROTOCOL".into());
                state.error_summary = Some(redact(&detail));
                break;
            }
            Err(ProtocolError::Io(e)) => {
                state.status = "failed".into();
                state.error_code = Some("E_IO".into());
                state.error_summary = Some(redact(&e));
                break;
            }
        }
        on_tick(conn, state);
    }
    Ok(())
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

    let mut source_meta = HashMap::new();
    let mut sources_json = Vec::new();
    for s in &request.sources {
        source_meta.insert(s.id.clone(), (s.path.clone(), s.kind.clone()));
        let cursor = load_cursor_json(&conn, &s.id)?;
        sources_json.push(serde_json::json!({
            "id": s.id,
            "kind": s.kind,
            "path": s.path,
            "cursor": cursor,
        }));
    }

    let config = serde_json::json!({
        "protocol": 1,
        "run_id": run_id,
        "sources": sources_json,
        "extractors": DEFAULT_EXTRACTORS,
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
        let result = drive_scan(&app2, &run_id2, &mut child, &cancel_file, source_meta);
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
    cancel_file: &Path,
    sources: HashMap<String, (String, String)>,
) -> Result<(), String> {
    let mut conn = db::connection(app)?;
    let scorer = StubScorer;
    let mut state = DriveState {
        stats: RunStats::default(),
        status: "completed".into(),
        error_code: None,
        error_summary: None,
        sources,
    };
    let mut last_emit = std::time::Instant::now()
        .checked_sub(std::time::Duration::from_secs(1))
        .unwrap_or_else(std::time::Instant::now);

    let child_handle = child.child_handle();
    let cancel_watch = cancel_file.to_path_buf();
    std::thread::spawn(move || watch_cancel_escalation(child_handle, cancel_watch));

    let stdout = child
        .stdout
        .as_mut()
        .ok_or_else(|| "sidecar stdout missing".to_string())?;

    consume_reader(
        &mut conn,
        run_id,
        stdout,
        &scorer,
        &mut state,
        |conn, st| {
            if last_emit.elapsed() >= std::time::Duration::from_millis(250) {
                let _ = update_run_stats(conn, run_id, &st.stats);
                let _ = app.emit(
                    "mail-scan://progress",
                    MailScanProgress {
                        run_id: run_id.to_string(),
                        listings_committed: st.stats.listings_committed,
                        messages_seen: st.stats.messages_seen,
                        status: "running".into(),
                    },
                );
                last_emit = std::time::Instant::now();
            }
        },
    )?;

    let exit = child.wait();
    match exit {
        Ok(Some(10)) => {
            if state.status == "completed" {
                state.status = "cancelled".into();
            }
        }
        Ok(Some(0)) => {}
        Ok(Some(code)) if state.status == "completed" => {
            state.status = "failed".into();
            state.error_code = state.error_code.or(Some(format!("E_EXIT_{code}")));
        }
        Ok(None) | Err(_) => {
            if state.status == "completed" {
                state.status = "failed".into();
                state.error_code = state.error_code.or(Some("E_CHILD".into()));
            }
        }
        Ok(Some(_)) => {}
    }

    let _ = update_run_stats(&mut conn, run_id, &state.stats);
    finish_run(
        &mut conn,
        run_id,
        &state.status,
        state.error_code.as_deref(),
        state.error_summary.as_deref(),
    )?;
    let _ = app.emit(
        "mail-scan://progress",
        MailScanProgress {
            run_id: run_id.to_string(),
            listings_committed: state.stats.listings_committed,
            messages_seen: state.stats.messages_seen,
            status: state.status.clone(),
        },
    );
    Ok(())
}

/// Process an in-memory NDJSON stream via the same dispatcher as production.
#[cfg(test)]
pub fn consume_event_stream<R: std::io::Read>(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    mut reader: R,
    scorer: &dyn ListingScorer,
) -> Result<RunStats, String> {
    start_run(conn, run_id)?;
    let mut state = DriveState {
        stats: RunStats::default(),
        status: "completed".into(),
        error_code: None,
        error_summary: None,
        sources: HashMap::new(),
    };
    consume_reader(conn, run_id, &mut reader, scorer, &mut state, |_, _| {})?;
    update_run_stats(conn, run_id, &state.stats)?;
    finish_run(
        conn,
        run_id,
        &state.status,
        state.error_code.as_deref(),
        state.error_summary.as_deref(),
    )?;
    Ok(state.stats)
}
