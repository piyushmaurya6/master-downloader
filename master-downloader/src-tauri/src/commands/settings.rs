use crate::db::repository::AppSettings;
use crate::error::{redact_sensitive_text, AppError, AppResult};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use tauri::State;
use zip::write::SimpleFileOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticLogEntry {
    pub line: String,
}

#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.db.get_settings()
}

#[tauri::command]
pub async fn update_app_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<AppSettings> {
    state.db.save_settings(&settings)?;
    state.worker.wake();
    Ok(settings)
}

#[tauri::command]
pub async fn pick_directory_dialog(default_dir: Option<String>) -> AppResult<Option<String>> {
    let _ = default_dir;
    #[cfg(target_os = "windows")]
    {
        let res = tokio::task::spawn_blocking(win32_browse_for_folder)
            .await
            .map_err(|e| AppError::Internal(format!("Folder dialog join error: {}", e)))?;
        return Ok(res);
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

#[cfg(target_os = "windows")]
fn win32_browse_for_folder() -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::ptr;

    #[repr(C)]
    struct BROWSEINFOW {
        hwnd_owner: *mut std::ffi::c_void,
        pidl_root: *const std::ffi::c_void,
        psz_display_name: *mut u16,
        lpsz_title: *const u16,
        ul_flags: u32,
        lpfn: *const std::ffi::c_void,
        l_param: isize,
        i_image: i32,
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHBrowseForFolderW(lpbi: *mut BROWSEINFOW) -> *mut std::ffi::c_void;
        fn SHGetPathFromIDListW(pidl: *const std::ffi::c_void, pszPath: *mut u16) -> i32;
    }

    #[link(name = "ole32")]
    extern "system" {
        fn CoInitializeEx(pvReserved: *mut std::ffi::c_void, dwCoInit: u32) -> i32;
        fn CoUninitialize();
        fn CoTaskMemFree(pv: *mut std::ffi::c_void);
    }

    const BIF_RETURNONLYFSDIRS: u32 = 0x00000001;
    const BIF_NEWDIALOGSTYLE: u32 = 0x00000040;
    const BIF_USENEWUI: u32 = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE | 0x00000010;
    const COINIT_APARTMENTTHREADED: u32 = 0x2;

    unsafe {
        let co_res = CoInitializeEx(ptr::null_mut(), COINIT_APARTMENTTHREADED);
        let mut display_name = [0u16; 260];
        let title_wide: Vec<u16> = std::ffi::OsStr::new("Select Master Downloader Output Folder")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut bi = BROWSEINFOW {
            hwnd_owner: ptr::null_mut(),
            pidl_root: ptr::null(),
            psz_display_name: display_name.as_mut_ptr(),
            lpsz_title: title_wide.as_ptr(),
            ul_flags: BIF_USENEWUI,
            lpfn: ptr::null(),
            l_param: 0,
            i_image: 0,
        };

        let pidl = SHBrowseForFolderW(&mut bi);
        let mut result = None;

        if !pidl.is_null() {
            let mut path_buf = [0u16; 260];
            if SHGetPathFromIDListW(pidl, path_buf.as_mut_ptr()) != 0 {
                let len = path_buf
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(path_buf.len());
                let os_str = OsString::from_wide(&path_buf[..len]);
                result = Some(os_str.to_string_lossy().to_string());
            }
            CoTaskMemFree(pidl);
        }

        if co_res >= 0 {
            CoUninitialize();
        }

        result
    }
}

#[tauri::command]
pub async fn get_diagnostic_logs(state: State<'_, AppState>) -> AppResult<Vec<DiagnosticLogEntry>> {
    let log_dir = state.app_data_dir.join("logs");
    let mut entries = Vec::new();
    if let Ok(dir) = fs::read_dir(&log_dir) {
        for item in dir.flatten() {
            if item.path().is_file() {
                if let Ok(content) = fs::read_to_string(item.path()) {
                    for line in content.lines().rev().take(250) {
                        entries.push(DiagnosticLogEntry {
                            line: redact_sensitive_text(line),
                        });
                    }
                }
            }
        }
    }
    Ok(entries)
}

#[tauri::command]
pub async fn export_diagnostic_bundle(state: State<'_, AppState>) -> AppResult<String> {
    let settings = state.db.get_settings().unwrap_or_default();
    let bin_status = state.binaries.check_status().await;

    let out_dir = dirs::download_dir().unwrap_or_else(|| state.app_data_dir.clone());
    let bundle_name = format!(
        "MasterDownloader_Diagnostics_{}.zip",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    let zip_path = out_dir.join(bundle_name);

    let file = File::create(&zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("settings.json", options)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let settings_json = serde_json::to_string_pretty(&settings).unwrap_or_default();
    zip.write_all(redact_sensitive_text(&settings_json).as_bytes())?;

    zip.start_file("binaries.json", options)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let bin_json = serde_json::to_string_pretty(&bin_status).unwrap_or_default();
    zip.write_all(bin_json.as_bytes())?;

    zip.finish()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(zip_path.to_string_lossy().to_string())
}
