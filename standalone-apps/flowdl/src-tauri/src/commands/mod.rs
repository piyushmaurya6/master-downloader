use crate::dependencies::dependency_manager::{
    check_all_dependencies, setup_or_repair_dependencies,
};
use crate::dependencies::engine_manager::EngineManager;
use crate::dependencies::ffmpeg_service::install_or_update_managed_ffmpeg;
use crate::dependencies::ytdlp_service::install_or_update_managed_ytdlp;
use crate::downloader::engine::{DownloadEngine, YtDlpEngine};
use crate::models::{
    AppSettings, AppUpdateInfo, DependencyState, DiagnosticLogEntry, DiagnosticReport,
    DownloadItem, DownloadRequest, EngineUpdateInfo, HistoryItem, MediaMetadata, ToolStatus,
};
use crate::platform::{open_file_in_os, pick_folder_dialog, reveal_in_explorer};
use crate::queue::queue_manager::QueueManager;
use crate::services::app_updater::check_for_app_update;
use crate::services::diagnostic_service::generate_sanitized_diagnostic_report;
use crate::storage::history_store::HistoryStore;
use crate::storage::settings_store::SettingsStore;
use crate::utils::{no_window_command, temp_dir, DiagnosticLogger};
use tauri::{AppHandle, State};

pub struct AppState {
    pub settings_store: SettingsStore,
    pub history_store: HistoryStore,
    pub queue_manager: QueueManager,
    pub logger: DiagnosticLogger,
}

#[tauri::command]
pub async fn check_dependencies(
    check_updates: bool,
    state: State<'_, AppState>,
) -> Result<DependencyState, String> {
    Ok(check_all_dependencies(&state.settings_store, check_updates, &state.logger).await)
}

#[tauri::command]
pub async fn setup_dependencies(
    force_download: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DependencyState, String> {
    let active = state.queue_manager.active_count();
    if !force_download {
        EngineManager::repair(Some(&app), &state.settings_store, active, &state.logger).await
    } else {
        setup_or_repair_dependencies(
            Some(&app),
            &state.settings_store,
            force_download,
            active,
            &state.logger,
        )
        .await
    }
}

#[tauri::command]
pub async fn update_ytdlp_engine(state: State<'_, AppState>) -> Result<ToolStatus, String> {
    if state.queue_manager.active_count() > 0 {
        return Err("Cannot update yt-dlp while a download is active.".to_string());
    }
    install_or_update_managed_ytdlp(&state.logger, |_, _| {}).await
}

#[tauri::command]
pub async fn update_ffmpeg_engine(state: State<'_, AppState>) -> Result<DependencyState, String> {
    if state.queue_manager.active_count() > 0 {
        return Err("Cannot update FFmpeg while a download is active.".to_string());
    }
    let _ = install_or_update_managed_ffmpeg(&state.logger, |_, _| {}).await?;
    Ok(check_all_dependencies(&state.settings_store, false, &state.logger).await)
}

#[tauri::command]
pub async fn check_engine_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EngineUpdateInfo, String> {
    EngineManager::check_updates(Some(&app), &state.settings_store, &state.logger).await
}

#[tauri::command]
pub async fn check_app_update(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppUpdateInfo, String> {
    check_for_app_update(Some(&app), &state.logger).await
}

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") && !trimmed.starts_with("http://") {
        return Err("Only http:// and https:// URLs can be opened externally.".to_string());
    }
    no_window_command("explorer.exe")
        .arg(trimmed)
        .spawn()
        .map_err(|e| format!("Failed to open URL in browser: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn analyze_url(url: String, state: State<'_, AppState>) -> Result<MediaMetadata, String> {
    let settings = state.settings_store.get();
    let engine = YtDlpEngine::new();
    engine.analyze_url(&url, &settings, &state.logger).await
}

#[tauri::command]
pub async fn enqueue_download(
    request: DownloadRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DownloadItem, String> {
    state.queue_manager.enqueue(Some(&app), request)
}

#[tauri::command]
pub async fn get_queue(state: State<'_, AppState>) -> Result<Vec<DownloadItem>, String> {
    Ok(state.queue_manager.list_queue())
}

#[tauri::command]
pub async fn cancel_download(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.queue_manager.cancel_or_stop(Some(&app), &id, "cancelled")
}

#[tauri::command]
pub async fn stop_download(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.queue_manager.cancel_or_stop(Some(&app), &id, "stopped")
}

#[tauri::command]
pub async fn resume_download(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.queue_manager.resume_or_retry(Some(&app), &id)
}

#[tauri::command]
pub async fn retry_download(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.queue_manager.resume_or_retry(Some(&app), &id)
}

#[tauri::command]
pub async fn remove_download(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.queue_manager.remove_item(Some(&app), &id)
}

#[tauri::command]
pub async fn clear_completed_downloads(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.queue_manager.clear_completed(Some(&app))
}

#[tauri::command]
pub async fn reorder_queue(
    id: String,
    direction: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.queue_manager.reorder_item(Some(&app), &id, &direction)
}

#[tauri::command]
pub async fn get_history(state: State<'_, AppState>) -> Result<Vec<HistoryItem>, String> {
    state.history_store.list_history()
}

#[tauri::command]
pub async fn remove_history_item(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.history_store.remove_history_entry(&id)
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.history_store.clear_history()
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings_store.get())
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let saved = state.settings_store.save(settings)?;
    state.queue_manager.schedule_next(Some(app));
    Ok(saved)
}

#[tauri::command]
pub async fn pick_folder(initial_dir: Option<String>) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || Ok(pick_folder_dialog(initial_dir)))
        .await
        .map_err(|e| format!("Failed to open native folder picker: {}", e))?
}

#[tauri::command]
pub async fn open_file(path: String) -> Result<(), String> {
    open_file_in_os(&path)
}

#[tauri::command]
pub async fn show_in_folder(path: String) -> Result<(), String> {
    reveal_in_explorer(&path)
}

#[tauri::command]
pub async fn get_diagnostic_logs(
    state: State<'_, AppState>,
) -> Result<Vec<DiagnosticLogEntry>, String> {
    Ok(state.logger.get_entries())
}

#[tauri::command]
pub async fn generate_diagnostic_report(
    state: State<'_, AppState>,
) -> Result<DiagnosticReport, String> {
    Ok(generate_sanitized_diagnostic_report(&state.settings_store, &state.logger).await)
}

#[tauri::command]
pub async fn clear_temporary_files(state: State<'_, AppState>) -> Result<(), String> {
    let t = temp_dir();
    let _ = std::fs::remove_dir_all(&t);
    let _ = std::fs::create_dir_all(&t);
    state.logger.info("Privacy", "Temporary files cleared.");
    Ok(())
}
