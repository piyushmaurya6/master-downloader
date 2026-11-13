use crate::core::binary_manager::{BinaryManager, BinaryStatusReport};
use crate::error::{AppError, AppResult};
use crate::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn check_binaries_status(state: State<'_, AppState>) -> AppResult<BinaryStatusReport> {
    Ok(state.binary_manager.inspect_status().await)
}

#[tauri::command]
pub async fn bootstrap_or_update_binaries(
    force_update: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<BinaryStatusReport> {
    state.log_buffer.push(
        "INFO",
        "BinaryManager",
        &format!(
            "Starting engine bootstrap/update (force_update={})...",
            force_update
        ),
    );
    let report = state
        .binary_manager
        .bootstrap_or_update_all(Some(&app), force_update)
        .await?;
    state.log_buffer.push(
        "INFO",
        "BinaryManager",
        &format!("Engine bootstrap complete (all_ready={}).", report.all_ready),
    );
    Ok(report)
}

#[tauri::command]
pub async fn verify_binary_sha256(
    binary_name: String,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let target = match binary_name.trim().to_lowercase().as_str() {
        "yt-dlp" | "yt-dlp.exe" => state.binary_manager.yt_dlp_path(),
        "gallery-dl" | "gallery-dl.exe" => state.binary_manager.gallery_dl_path(),
        "ffmpeg" | "ffmpeg.exe" => state.binary_manager.ffmpeg_path(),
        "ffprobe" | "ffprobe.exe" => state.binary_manager.ffprobe_path(),
        other => return Err(AppError::BinaryMissing(other.to_string())),
    };

    if !target.exists() {
        return Err(AppError::BinaryMissing(binary_name));
    }

    BinaryManager::compute_file_sha256(&target)
}
