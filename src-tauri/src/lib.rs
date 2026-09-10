mod calendar;
mod db;
mod google_oauth;
mod ipc_responsiveness;
mod job_search;
mod listing_check;
mod llm;
mod mail_scan;
mod migrations;
mod net;
mod secrets;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(mail_scan::MailScanRuntime::default())
        .setup(|app| {
            if let Err(e) = secrets::init_app_store(app.handle()) {
                log::warn!("Secret store init failed: {}", secrets::redact(&e));
            }
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
            db::init_db,
            db::create_job,
            db::list_jobs,
            db::delete_job,
            db::update_job,
            db::update_job_status,
            db::list_status_history,
            db::save_application_pdf,
            db::list_job_documents,
            db::save_job_document,
            db::delete_job_document,
            db::import_jobs,
            db::backup_to_folder,
            db::open_document,
            secrets::llm_key_set,
            secrets::llm_key_status,
            secrets::llm_key_clear,
            llm::client::extract_job_info,
            llm::client::llm_test_connection,
            llm::overrides::llm_provider_override_get,
            llm::overrides::llm_provider_override_set,
            calendar::google_calendar_create_event,
            google_oauth::google_oauth_get_client_id,
            google_oauth::google_oauth_set_client_id,
            google_oauth::google_oauth_status,
            google_oauth::google_oauth_connect,
            google_oauth::google_oauth_disconnect,
            job_search::get_keyword_stats,
            job_search::get_location_suggestions,
            job_search::fetch_job_search_results,
            job_search::fetch_job_search_bundle,
            job_search::build_search_url,
            job_search::fetch_job_search_result_page_text,
            job_search::open_url_in_browser,
            listing_check::check_listing_status,
            mail_scan::mail_scan_start,
            mail_scan::mail_scan_cancel,
            mail_scan::mail_scan_estimate,
            mail_scan::settings::mail_scan_settings_get,
            mail_scan::settings::mail_scan_settings_set,
            mail_scan::settings::mail_scan_resolve_sources,
            mail_scan::settings::mail_scan_test_source,
            mail_scan::settings::mail_scan_detect_thunderbird,
            mail_scan::settings::mail_scan_pick_path,
            mail_scan::settings::mail_scan_sidecar_probe,
            mail_scan::settings::mail_scan_delete_all_data,
            mail_scan::inbox::mail_match_list,
            mail_scan::inbox::mail_match_list_dismissed,
            mail_scan::inbox::mail_match_sightings,
            mail_scan::inbox::mail_match_dismiss,
            mail_scan::inbox::mail_match_restore,
            mail_scan::inbox::mail_scan_list_runs,
            mail_scan::accept::mail_match_preview_update,
            mail_scan::accept::mail_match_accept_update,
            mail_scan::accept::mail_match_accept_new,
            mail_scan::profiles::mail_scan_profile_status,
            mail_scan::profiles::mail_scan_profile_set_from_path,
            mail_scan::profiles::mail_scan_profile_clear,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
