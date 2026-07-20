pub mod backup;
pub mod commands;
pub mod daily_queue;
pub mod integration;
pub mod learning;
pub mod problems;
pub mod settings;
pub mod storage;

use commands::{AppInner, AppState};
use learning::FsrsScheduler;
use std::sync::{Arc, Mutex};
use storage::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("app data directory");
            std::fs::create_dir_all(&data_dir).expect("create app data directory");
            let db_path = data_dir.join("ankicode.sqlite");
            let db = Database::open(&db_path).expect("open ankicode database");
            let settings = db.get_settings().expect("load settings");
            let scheduler = FsrsScheduler::new(settings.desired_retention as f32)
                .expect("build scheduler from settings");
            let state = AppState {
                inner: Arc::new(Mutex::new(AppInner { db, scheduler })),
            };
            let server_state = state.clone();
            app.manage(state);
            tauri::async_runtime::spawn(async move {
                if let Err(error) = integration::serve(server_state).await {
                    eprintln!("Ankicode loopback API failed: {error}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::complete_onboarding,
            commands::get_today,
            commands::list_problems_view,
            commands::add_problem_from_url,
            commands::set_problem_status_cmd,
            commands::get_problem_detail,
            commands::record_rating,
            commands::list_pending_completions,
            commands::get_loopback_status,
            commands::update_settings,
            commands::regenerate_pairing_code,
            commands::export_backup,
            commands::import_backup,
            commands::open_problem_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
