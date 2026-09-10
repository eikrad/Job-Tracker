//! Guardrail: network / long I/O must not run as sync Tauri commands.
//!
//! Sync `#[tauri::command]` handlers execute inline on the IPC dispatch path
//! (Tauri macros default `ExecutionContext::Blocking`). That freezes the whole
//! window for the duration of the call — the user-visible hang when starting a
//! job search or saving a date (backup / calendar / LLM).
//!
//! Pattern to follow: `async fn` + `tauri::async_runtime::spawn_blocking` for
//! `reqwest::blocking` / vacuum / other multi-second work (see `listing_check`).

#[cfg(test)]
mod tests {
    fn assert_async_command(src: &str, name: &str) {
        let needle = format!("pub async fn {name}");
        assert!(
            src.contains(&needle),
            "{name} must be `pub async fn` so long I/O cannot freeze the UI (found sync command)"
        );
    }

    #[test]
    fn network_and_heavy_io_commands_are_async() {
        let job_search = include_str!("job_search.rs");
        assert_async_command(job_search, "fetch_job_search_bundle");
        assert_async_command(job_search, "fetch_job_search_results");
        assert_async_command(job_search, "fetch_job_search_result_page_text");

        let calendar = include_str!("calendar.rs");
        assert_async_command(calendar, "google_calendar_create_event");

        let db = include_str!("db.rs");
        assert_async_command(db, "backup_to_folder");

        let llm = include_str!("llm/client.rs");
        assert_async_command(llm, "llm_test_connection");
        assert_async_command(llm, "extract_job_info");
    }

    #[test]
    fn google_http_clients_set_timeouts() {
        // `Client::new()` has no timeout — hangs look like a frozen app.
        let oauth = include_str!("google_oauth.rs");
        let calendar = include_str!("calendar.rs");
        assert!(
            !oauth.contains("reqwest::blocking::Client::new()"),
            "google_oauth must not use Client::new() without timeouts"
        );
        assert!(
            !calendar.contains("reqwest::blocking::Client::new()"),
            "calendar must not use Client::new() without timeouts"
        );
    }
}
