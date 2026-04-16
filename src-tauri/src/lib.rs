mod calendar;
mod db;
mod google_oauth;
mod job_search;

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
            job_search::open_url_in_browser,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
