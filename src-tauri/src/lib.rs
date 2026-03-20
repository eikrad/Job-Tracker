mod calendar;
mod db;

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
      db::update_job_status,
      db::list_status_history,
      db::save_application_pdf,
      db::import_jobs,
      calendar::google_calendar_create_event,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
