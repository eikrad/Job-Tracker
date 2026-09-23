//! Mail-scan orchestration: sidecar spawn, NDJSON protocol, scoring, persistence.

pub mod accept;
pub mod budget;
pub mod cluster;
pub mod digest_split;
#[cfg(test)]
mod corpus;
pub mod enrichment;
pub mod inbox;
pub mod injection;
pub mod listing_page;
pub mod persist;
pub mod profiles;
pub mod protocol;
pub mod score_cache;
pub mod settings;
pub mod sidecar;
pub mod scoring;
pub mod spawn;
pub mod status;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rusqlite::OptionalExtension;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::db;
use crate::secrets::redact;

use self::persist::{
    finish_run, gate_listing, persist_listing, record_run_identity, start_run, update_run_stats,
    upsert_source_cursor, RunStats, ScoringIdentities,
};
use self::enrichment::{Enrichment, LlmEnricher};
use self::profiles::{load_profile, ProfileKind};
use self::digest_split::LlmSplitter;
use self::protocol::{
    read_event_line, Event, ListingEvent, ProtocolError, MAX_LINE_BYTES, PROTOCOL,
};
use self::scoring::{LlmScorer, RunStop, ScoringConfig, ScoringEngine};
use self::settings::MailSource;
use self::status::{EnrichmentState, RunStatus, SourceKind, Verdict};
use self::spawn::{spawn_scan, watch_cancel_escalation, SpawnedScan};
use crate::llm::overrides::resolved_spec;
use crate::llm::provider::LlmProvider;
use crate::secrets;

/// Extractor names shipped with the sidecar, in dispatch order (pinned to Python
/// `DEFAULT_EXTRACTORS` by `tests/test_mail_scan_sidecar.py`).
pub const DEFAULT_EXTRACTORS: &[&str] = &["jobindex", "linkedin", "indeed", "digest"];

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
    pub status: RunStatus,
}

struct DriveState {
    stats: RunStats,
    status: RunStatus,
    error_code: Option<String>,
    error_summary: Option<String>,
    /// The configured folders by id, for cursor commits.
    sources: HashMap<String, MailSource>,
    /// Listings held back so pass 1 can be batched (spec §8.3). Bounded by the batch
    /// size, and each one is still committed in its own transaction after scoring.
    buffer: Vec<ListingEvent>,
}

impl DriveState {
    fn new(sources: HashMap<String, MailSource>) -> Self {
        Self {
            stats: RunStats::default(),
            status: RunStatus::Completed,
            error_code: None,
            error_summary: None,
            sources,
            buffer: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, summary: String) {
        self.status = RunStatus::Failed;
        self.error_code = Some(code.to_string());
        self.error_summary = Some(redact(&summary));
    }
}

fn new_run_id() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let suffix: String = (0..8).map(|_| format!("{:02x}", rng.random::<u8>())).collect();
    format!("ms_{}_{}", chrono::Utc::now().timestamp_millis(), suffix)
}

pub(crate) fn python_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../python")
}

fn sidecar_python_override() -> Option<PathBuf> {
    std::env::var_os("JOBTRACKER_MAIL_SCAN_PYTHON").map(PathBuf::from)
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// What a run scored with — recorded on the run row so a score stays explainable.
struct EngineIdentity {
    model_id: String,
    profile_short_hash: String,
    profile_full_hash: String,
}

/// Assemble the scorer. Deliberately called *before* the sidecar is spawned: a missing
/// profile or key should fail in under a second, not after reading a 200 MB mbox.
fn build_engine(
    app: &AppHandle,
    provider: LlmProvider,
    config: ScoringConfig,
) -> Result<(ScoringEngine, EngineIdentity), String> {
    let dir = app_data_dir(app)?;
    let short = load_profile(&dir, ProfileKind::Short)?;
    let full = load_profile(&dir, ProfileKind::Full)?;
    let key = secrets::get_secret(provider.secret_provider())?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            "E_CONFIG_INCOMPLETE: add an API key for the scoring provider in Settings.".to_string()
        })?;
    let spec = resolved_spec(app, provider)?;
    let identity = EngineIdentity {
        model_id: spec.model_id.clone(),
        profile_short_hash: short.content_hash.clone(),
        profile_full_hash: full.content_hash.clone(),
    };
    let scorer = LlmScorer::new(spec.clone(), key.clone(), short, full);
    let splitter = LlmSplitter::new(spec.clone(), key.clone());
    let enricher = LlmEnricher::new(spec, key);
    Ok((
        ScoringEngine::new(Box::new(scorer), config)
            .with_enricher(Box::new(enricher))
            .with_splitter(Box::new(splitter)),
        identity,
    ))
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

/// Score the buffered listings and commit each one. Returns false when the run must
/// stop issuing calls.
///
/// Listings that could not be scored (budget, breaker) are **not** persisted: leaving
/// them untouched means the next run picks them up, whereas persisting them with a
/// placeholder score would cache a non-answer as a verdict.
fn flush_buffer(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    engine: &mut ScoringEngine,
    state: &mut DriveState,
) -> Result<bool, String> {
    if state.buffer.is_empty() {
        return Ok(true);
    }
    let buffered = std::mem::take(&mut state.buffer);
    let refs: Vec<&ListingEvent> = buffered.iter().collect();
    let batch = engine.score_batch(conn, &refs)?;
    let identities = ScoringIdentities {
        pass1: engine.identity(1),
        pass2: engine.identity(2),
    };

    for (listing, scored) in buffered.iter().zip(batch.results.iter()) {
        let Some(scored) = scored else { continue };
        // Only what reached the inbox is worth a fetch; under-cutoff listings are
        // recorded as sightings and never enriched.
        let enrichment = if scored.outcome == Verdict::Inbox {
            engine.enrich(listing)
        } else {
            Enrichment::skipped()
        };
        if enrichment.state == EnrichmentState::Failed {
            state.stats.enrichment_failures += 1;
        }
        match persist_listing(conn, run_id, listing, scored, &enrichment, &identities) {
            Ok(outcome) => state.stats.count(outcome),
            Err(e) => {
                state.fail("E_DB", e);
                return Ok(false);
            }
        }
    }
    state.stats.llm_calls = engine.budget().calls_used();
    Ok(continue_after(batch.stop, run_id, state))
}

/// Queue a listing for scoring, unless the gate already settled it. Returns false when
/// the run must stop.
///
/// The gate runs here, before the listing takes a place in a pass-1 batch: a listing
/// that is dismissed or already a Job costs no model call and no page fetch.
fn admit_listing(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    listing: ListingEvent,
    engine: &mut ScoringEngine,
    state: &mut DriveState,
) -> Result<bool, String> {
    match gate_listing(conn, &listing) {
        Ok(Some(outcome)) => {
            state.stats.count(outcome);
            return Ok(true);
        }
        Ok(None) => {}
        Err(e) => {
            state.fail("E_DB", e);
            return Ok(false);
        }
    }
    state.buffer.push(listing);
    if state.buffer.len() >= engine.batch_size() {
        return flush_buffer(conn, run_id, engine, state);
    }
    Ok(true)
}

/// Apply a stop signal from scoring or splitting. Returns false when the run must stop.
fn continue_after(stop: Option<RunStop>, run_id: &str, state: &mut DriveState) -> bool {
    match stop {
        None => true,
        // The cap is a successful stop: the run completes, keeps its results, and the
        // UI offers Continue. Treating it as a failure would train the user to ignore
        // failures.
        Some(RunStop::BudgetExhausted) => {
            state.stats.budget_exhausted = true;
            log::info!("mail scan {run_id}: call budget exhausted, stopping cleanly");
            false
        }
        Some(RunStop::Unavailable(detail)) => {
            state.fail("E_LLM_UNAVAILABLE", detail);
            false
        }
        Some(RunStop::Fatal { code, detail }) => {
            state.fail(code, detail);
            false
        }
    }
}

/// Apply one protocol event to DB/stats. Returns false when the run should stop.
fn apply_event(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    event: Event,
    engine: &mut ScoringEngine,
    state: &mut DriveState,
) -> Result<bool, String> {
    match event {
        Event::Unknown { t } => {
            log::info!("mail scan: skipping unknown event t={t}");
            Ok(true)
        }
        Event::Started(ev) => {
            if ev.protocol != PROTOCOL {
                state.status = RunStatus::Failed;
                state.error_code = Some("E_PROTOCOL_MISMATCH".into());
                state.error_summary = Some(format!(
                    "protocol mismatch: sidecar={}, expected={PROTOCOL}",
                    ev.protocol
                ));
                return Ok(false);
            }
            Ok(true)
        }
        Event::Listing(listing) => admit_listing(conn, run_id, *listing, engine, state),
        Event::Digest(digest) => {
            // Split listings join the same buffer as extracted ones, so they pass the
            // same dismissal gate, scoring, enrichment, and per-item commit.
            let split = engine.split_digest(conn, run_id, &digest)?;
            state.stats.llm_calls = engine.budget().calls_used();
            for listing in split.listings {
                if !admit_listing(conn, run_id, listing, engine, state)? {
                    return Ok(false);
                }
            }
            Ok(continue_after(split.stop, run_id, state))
        }
        Event::SourceFinished(ev) => {
            // Commit the cursor only after everything read from this source is on
            // disk. Advancing it over buffered listings would skip them for good on
            // the next incremental run.
            if !flush_buffer(conn, run_id, engine, state)? {
                return Ok(false);
            }
            let source = state.sources.get(&ev.source);
            let _ = upsert_source_cursor(
                conn,
                &ev.source,
                source.map_or("", |s| s.path.as_str()),
                source.map_or(SourceKind::Mbox, |s| s.kind),
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
                state.status = RunStatus::Cancelled;
            }
            state.stats.listing_limit_reached = ev.truncated.unwrap_or(false);
            Ok(true)
        }
    }
}

fn consume_reader<R: std::io::Read>(
    conn: &mut rusqlite::Connection,
    run_id: &str,
    reader: &mut R,
    engine: &mut ScoringEngine,
    state: &mut DriveState,
    mut on_tick: impl FnMut(&mut rusqlite::Connection, &DriveState),
) -> Result<(), String> {
    let mut line_buf: Vec<u8> = Vec::with_capacity(4096);
    let mut stopped = false;
    loop {
        match read_event_line(reader, &mut line_buf, MAX_LINE_BYTES) {
            Ok(None) => break,
            Ok(Some(event)) => {
                if !apply_event(conn, run_id, event, engine, state)? {
                    stopped = true;
                    break;
                }
            }
            Err(ProtocolError::Oversize) => {
                state.status = RunStatus::Failed;
                state.error_code = Some("E_PROTOCOL_OVERSIZE".into());
                state.error_summary = Some("NDJSON line exceeded 256 KiB".into());
                break;
            }
            Err(ProtocolError::Malformed(detail)) => {
                state.status = RunStatus::Failed;
                state.error_code = Some("E_PROTOCOL".into());
                state.error_summary = Some(redact(&detail));
                break;
            }
            Err(ProtocolError::Io(e)) => {
                state.status = RunStatus::Failed;
                state.error_code = Some("E_IO".into());
                state.error_summary = Some(redact(&e));
                break;
            }
        }
        on_tick(conn, state);
    }
    // Score whatever is still buffered, even when the stream died mid-way: those
    // listings were read successfully and throwing them away would mean a truncated
    // stream costs the user work it had already paid for.
    if !stopped {
        flush_buffer(conn, run_id, engine, state)?;
    } else if !state.buffer.is_empty() {
        let salvage = std::mem::take(&mut state.buffer);
        log::info!(
            "mail scan {run_id}: stopped with {} listing(s) unscored; next run picks them up",
            salvage.len()
        );
    }
    on_tick(conn, state);
    Ok(())
}

/// Start a scan with the saved Settings. The only thing a caller chooses is
/// `force_rescore`, the explicit "Re-score backlog" action.
#[tauri::command]
pub fn mail_scan_start(
    app: AppHandle,
    runtime: State<'_, MailScanRuntime>,
    force_rescore: Option<bool>,
) -> Result<String, String> {
    {
        let guard = runtime.inner.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("A mail scan is already running.".into());
        }
    }

    // Fail before doing any work if the profiles or the key are missing — a scan that
    // reads a whole mail folder and only then discovers it cannot score is worse than
    // one that never starts.
    let settings = settings::load_settings(&app)?;
    let (engine, engine_identity) = build_engine(
        &app,
        settings.provider()?,
        settings.scoring_config(force_rescore.unwrap_or(false)),
    )?;

    let run_id = new_run_id();
    let cancel_dir = std::env::temp_dir().join("jobtracker-mail-scan");
    std::fs::create_dir_all(&cancel_dir).map_err(|e| e.to_string())?;
    let cancel_file = cancel_dir.join(format!("{run_id}.cancel"));
    let _ = std::fs::remove_file(&cancel_file);

    let mut conn = db::connection(&app)?;
    start_run(&mut conn, &run_id)?;
    record_run_identity(
        &conn,
        &run_id,
        &engine_identity.model_id,
        &engine_identity.profile_short_hash,
        &engine_identity.profile_full_hash,
    )?;

    let mut source_meta = HashMap::new();
    let mut sources_json = Vec::new();
    for s in &settings.sources {
        source_meta.insert(s.id.clone(), s.clone());
        let cursor = load_cursor_json(&conn, &s.id)?;
        sources_json.push(serde_json::json!({
            "id": s.id,
            "kind": s.kind,
            "path": s.path,
            "cursor": cursor,
        }));
    }

    let config = serde_json::json!({
        "protocol": PROTOCOL,
        "run_id": run_id,
        "sources": sources_json,
        "extractors": DEFAULT_EXTRACTORS,
        "limits": {
            "max_messages_per_source": 5000,
            "max_message_bytes": 2_097_152,
            "max_listings_per_run": 2000,
            "max_body_chars": 20_000,
        },
        "since": settings.since_iso(),
        "cancel_file": cancel_file.to_string_lossy(),
    });

    let mode = sidecar::resolve(&python_root(), sidecar_python_override())?;
    log::info!("mail scan {run_id}: using {}", mode.describe());
    let mut child = match spawn_scan(&mode, &python_root(), &config.to_string()) {
        Ok(c) => c,
        Err(e) => {
            let mut c = db::connection(&app)?;
            let _ = finish_run(&mut c, &run_id, RunStatus::Failed, Some("E_SPAWN"), Some(&redact(&e)));
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
        let mut engine = engine;
        let result = drive_scan(
            &app2,
            &run_id2,
            &mut child,
            &cancel_file,
            source_meta,
            &mut engine,
        );
        let mut guard = runtime2.lock().unwrap_or_else(|e| e.into_inner());
        *guard = None;
        if let Err(e) = result {
            log::warn!("mail scan ended with error: {}", redact(&e));
        }
    });

    Ok(run_id)
}

/// Pre-run cost preview and re-score backlog size (spec §8.3, §11.3).
///
/// Shown before the user commits to spending: how many calls a scan would cost, and
/// how many under-cutoff listings a "Re-score backlog" would re-pay for.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanEstimate {
    pub estimate: budget::CallEstimate,
    pub backlog_under_cutoff: i64,
    pub model_id: String,
    pub profile_short: profiles::ProfileStatus,
    pub profile_full: profiles::ProfileStatus,
}

#[tauri::command]
pub fn mail_scan_estimate(
    app: AppHandle,
    expected_listings: Option<u32>,
) -> Result<ScanEstimate, String> {
    let settings = settings::load_settings(&app)?;
    let dir = app_data_dir(&app)?;
    let conn = db::connection(&app)?;

    Ok(ScanEstimate {
        estimate: budget::estimate_calls(
            expected_listings.unwrap_or(0),
            scoring::PASS1_BATCH_SIZE,
            // Coarse prior for the share clearing the pass-1 gate; shown as an
            // estimate, never billed against.
            0.34,
            settings.max_calls,
        ),
        backlog_under_cutoff: score_cache::count_under_cutoff(&conn)?,
        model_id: resolved_spec(&app, settings.provider()?)?.model_id,
        profile_short: profiles::profile_status(&dir, ProfileKind::Short),
        profile_full: profiles::profile_status(&dir, ProfileKind::Full),
    })
}

#[tauri::command]
pub fn mail_scan_cancel(runtime: State<'_, MailScanRuntime>) -> Result<(), String> {
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
    sources: HashMap<String, MailSource>,
    engine: &mut ScoringEngine,
) -> Result<(), String> {
    let mut conn = db::connection(app)?;
    let mut state = DriveState::new(sources);
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
        engine,
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
                        status: RunStatus::Running,
                    },
                );
                last_emit = std::time::Instant::now();
            }
        },
    )?;

    let exit = child.wait();
    match exit {
        Ok(Some(10)) => {
            if state.status == RunStatus::Completed {
                state.status = RunStatus::Cancelled;
            }
        }
        Ok(Some(0)) => {}
        Ok(Some(code)) if state.status == RunStatus::Completed => {
            state.status = RunStatus::Failed;
            state.error_code = state.error_code.or(Some(format!("E_EXIT_{code}")));
        }
        Ok(None) | Err(_) => {
            if state.status == RunStatus::Completed {
                state.status = RunStatus::Failed;
                state.error_code = state.error_code.or(Some("E_CHILD".into()));
            }
        }
        Ok(Some(_)) => {}
    }

    let _ = update_run_stats(&mut conn, run_id, &state.stats);
    finish_run(
        &mut conn,
        run_id,
        state.status,
        state.error_code.as_deref(),
        state.error_summary.as_deref(),
    )?;
    let _ = app.emit(
        "mail-scan://progress",
        MailScanProgress {
            run_id: run_id.to_string(),
            listings_committed: state.stats.listings_committed,
            messages_seen: state.stats.messages_seen,
            status: state.status,
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
    engine: &mut ScoringEngine,
) -> Result<RunStats, String> {
    start_run(conn, run_id)?;
    let mut state = DriveState::new(HashMap::new());
    consume_reader(conn, run_id, &mut reader, engine, &mut state, |_, _| {})?;
    update_run_stats(conn, run_id, &state.stats)?;
    finish_run(
        conn,
        run_id,
        state.status,
        state.error_code.as_deref(),
        state.error_summary.as_deref(),
    )?;
    Ok(state.stats)
}
