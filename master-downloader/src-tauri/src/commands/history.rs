use crate::core::binary_manager::CREATE_NO_WINDOW;
use crate::db::repository::{HistoryFilter, JobRecord};
use crate::error::{AppError, AppResult};
use crate::AppState;
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn query_download_history(
    state: State<'_, AppState>,
    filter: HistoryFilter,
) -> AppResult<Vec<JobRecord>> {
    state.db.query_history(&filter)
}

#[tauri::command]
pub async fn delete_history_item(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: String,
    delete_files: bool,
) -> AppResult<()> {
    state.processes.abort_and_kill_job(&job_id);
    let paths = state.db.delete_job(&job_id)?;
    if delete_files {
        for p in paths {
            let path = Path::new(&p);
            if path.exists() {
                let _ = fs::remove_file(path);
            }
        }
    }
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}

#[tauri::command]
pub async fn reveal_in_explorer(file_path: String) -> AppResult<()> {
    let normalized = file_path.replace('/', "\\");
    let path = Path::new(&normalized);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        if path.exists() {
            if path.is_dir() {
                std::process::Command::new("explorer.exe")
                    .arg(&normalized)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()?;
            } else {
                std::process::Command::new("explorer.exe")
                    .raw_arg(format!("/select,\"{}\"", normalized))
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
                return Err(AppError::Io("Target folder no longer exists on disk.".into()));
            }
        } else {
            return Err(AppError::Io("Target file no longer exists on disk.".into()));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn open_media_file(file_path: String) -> AppResult<()> {
    let normalized = file_path.replace('/', "\\");
    let path = Path::new(&normalized);
    if !path.exists() {
        return Err(AppError::Io("Media file no longer exists on disk.".into()));
    }
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        #[link(name = "shell32")]
        extern "system" {
            fn ShellExecuteW(
                hwnd: *mut std::ffi::c_void,
                lpOperation: *const u16,
                lpFile: *const u16,
                lpParameters: *const u16,
                lpDirectory: *const u16,
                nShowCmd: i32,
            ) -> isize;
        }

        const SW_SHOWNORMAL: i32 = 1;
        let op_wide: Vec<u16> = OsStr::new("open")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let file_wide: Vec<u16> = OsStr::new(&normalized)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let code = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                op_wide.as_ptr(),
                file_wide.as_ptr(),
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if code <= 32 {
            return Err(AppError::Io(format!(
                "Windows Shell failed to open file (code {}).",
                code
            )));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn export_history_json(
    state: State<'_, AppState>,
    export_path: String,
) -> AppResult<usize> {
    let all = state.db.query_history(&HistoryFilter {
        search_query: None,
        platform: None,
        media_type: None,
        status: None,
        limit: Some(5000),
        offset: Some(0),
    })?;
    let count = all.len();
    let json = serde_json::to_string_pretty(&all)
        .map_err(|e| AppError::Internal(format!("Serialize history error: {}", e)))?;
    fs::write(&export_path, json)?;
    Ok(count)
}
