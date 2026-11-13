use crate::core::binary_manager::{compute_file_sha256, BinaryStatusReport};
use crate::error::{AppError, AppResult};
use crate::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn check_binaries_status(state: State<'_, AppState>) -> AppResult<BinaryStatusReport> {
    Ok(state.binaries.check_status().await)
}

#[tauri::command]
pub async fn bootstrap_or_update_binaries(
    state: State<'_, AppState>,
    app: AppHandle,
    force_update: bool,
) -> AppResult<BinaryStatusReport> {
    state.binaries.bootstrap_all(Some(&app), force_update).await
}

#[tauri::command]
pub async fn verify_binary_sha256(
    state: State<'_, AppState>,
    binary_name: String,
) -> AppResult<String> {
    let path = state.binaries.bin_dir.join(&binary_name);
    if !path.exists() {
        return Err(AppError::BinaryMissing(format!(
            "{} is not installed.",
            binary_name
        )));
    }
    compute_file_sha256(&path)
}
