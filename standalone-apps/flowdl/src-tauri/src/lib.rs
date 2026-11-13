pub mod commands;
pub mod dependencies;
pub mod downloader;
pub mod models;
pub mod platform;
pub mod queue;
pub mod services;
pub mod storage;
pub mod utils;

use commands::AppState;
use queue::queue_manager::QueueManager;
use storage::history_store::HistoryStore;
use storage::settings_store::SettingsStore;
use utils::DiagnosticLogger;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let logger = DiagnosticLogger::new();
    logger.info("System", "FlowDL starting up...");
    utils::ensure_bundled_binaries_seeded(&logger);

    let settings_store = SettingsStore::new();
    let history_store = HistoryStore::new().expect("Failed to initialize FlowDL SQLite database");
    let queue_manager = QueueManager::new(
        settings_store.clone(),
        history_store.clone(),
        logger.clone(),
    );

    let app_state = AppState {
        settings_store,
        history_store,
        queue_manager,
        logger,
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::check_dependencies,
            commands::setup_dependencies,
            commands::update_ytdlp_engine,
            commands::update_ffmpeg_engine,
            commands::check_engine_updates,
            commands::check_app_update,
            commands::open_external_url,
            commands::analyze_url,
            commands::enqueue_download,
            commands::get_queue,
            commands::cancel_download,
            commands::stop_download,
            commands::resume_download,
            commands::retry_download,
            commands::remove_download,
            commands::clear_completed_downloads,
            commands::reorder_queue,
            commands::get_history,
            commands::remove_history_item,
            commands::clear_history,
            commands::get_settings,
            commands::save_settings,
            commands::pick_folder,
            commands::open_file,
            commands::show_in_folder,
            commands::get_diagnostic_logs,
            commands::generate_diagnostic_report,
            commands::clear_temporary_files,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FlowDL desktop application");
}
