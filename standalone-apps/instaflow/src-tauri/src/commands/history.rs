use crate::core::binary_manager::CREATE_NO_WINDOW;
use crate::db::{HistoryFilter, JobRecord};
use crate::error::{AppError, AppResult};
use crate::AppState;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn query_download_history(
    filter: HistoryFilter,
    state: State<'_, AppState>,
) -> AppResult<Vec<JobRecord>> {
    state.repo.query_history(&filter)
}

#[tauri::command]
pub async fn delete_history_item(
    job_id: String,
    delete_files: bool,
    state: State<'_, AppState>,
) -> AppResult<()> {
    state.repo.delete_job(&job_id, delete_files)
}

#[tauri::command]
pub async fn reveal_in_explorer(file_path: String) -> AppResult<()> {
    let path = Path::new(&file_path);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        if path.exists() {
            if path.is_dir() {
                std::process::Command::new("explorer.exe")
                    .arg(path)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()?;
            } else {
                std::process::Command::new("explorer.exe")
                    .arg("/select,")
                    .arg(path)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()?;
            }
        } else if let Some(parent) = path.parent() {
            if parent.exists() {
                std::process::Command::new("explorer.exe")
                    .arg(parent)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()?;
            } else {
                return Err(AppError::MediaDeletedOrUnavailable);
            }
        } else {
            return Err(AppError::MediaDeletedOrUnavailable);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn open_media_file(file_path: String) -> AppResult<()> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(AppError::MediaDeletedOrUnavailable);
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", &file_path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn export_history_json(
    export_path: String,
    state: State<'_, AppState>,
) -> AppResult<usize> {
    let records = state.repo.query_history(&HistoryFilter::default())?;
    let json = serde_json::to_string_pretty(&records)
        .map_err(|e| AppError::ProcessError(e.to_string()))?;
    std::fs::write(&export_path, json)?;
    Ok(records.len())
}
