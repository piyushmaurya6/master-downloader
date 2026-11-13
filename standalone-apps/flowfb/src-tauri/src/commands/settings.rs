use crate::core::binary_manager::CREATE_NO_WINDOW;
use crate::core::queue_worker::DiagnosticLogEntry;
use crate::db::AppSettings;
use crate::error::{redact_sensitive, AppResult};
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct DiagnosticBundle {
    pub app_version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub settings_redacted: AppSettings,
    pub binaries: crate::core::binary_manager::BinaryStatusReport,
    pub logs: Vec<DiagnosticLogEntry>,
}

#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.repo.get_settings()
}

#[tauri::command]
pub async fn update_app_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> AppResult<AppSettings> {
    let updated = state.repo.save_settings(settings)?;
    state.queue_worker.notify_new_work();
    Ok(updated)
}

#[tauri::command]
pub async fn pick_directory_dialog(default_dir: Option<String>) -> AppResult<Option<String>> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let initial = default_dir.unwrap_or_default().replace('\'', "''");
        let ps_script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.FolderBrowserDialog; $f.SelectedPath = '{}'; if ($f.ShowDialog() -eq 'OK') {{ Write-Output $f.SelectedPath }}",
            initial
        );
        let out = tokio::task::spawn_blocking(move || {
            std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
        })
        .await
        .map_err(|e| crate::error::AppError::ProcessError(e.to_string()))??;

        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if stdout.is_empty() {
            Ok(None)
        } else {
            Ok(Some(stdout))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = default_dir;
        Ok(None)
    }
}

#[tauri::command]
pub async fn get_diagnostic_logs(state: State<'_, AppState>) -> AppResult<Vec<DiagnosticLogEntry>> {
    Ok(state.log_buffer.snapshot())
}

#[tauri::command]
pub async fn export_diagnostic_bundle(state: State<'_, AppState>) -> AppResult<String> {
    let mut settings = state.repo.get_settings()?;
    settings.output_dir = redact_sensitive(&settings.output_dir);
    settings.connected_username = settings
        .connected_username
        .map(|_| "<REDACTED_ACCOUNT>".to_string());
    settings.proxy_url = settings.proxy_url.map(|_| "<REDACTED_PROXY>".to_string());

    let mut binaries = state.binary_manager.inspect_status().await;
    binaries.bin_dir = redact_sensitive(&binaries.bin_dir);
    binaries.yt_dlp.path = redact_sensitive(&binaries.yt_dlp.path);
    binaries.gallery_dl.path = redact_sensitive(&binaries.gallery_dl.path);
    binaries.ffmpeg.path = redact_sensitive(&binaries.ffmpeg.path);
    binaries.ffprobe.path = redact_sensitive(&binaries.ffprobe.path);

    let bundle = DiagnosticBundle {
        app_version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        settings_redacted: settings,
        binaries,
        logs: state.log_buffer.snapshot(),
    };

    serde_json::to_string_pretty(&bundle)
        .map_err(|e| crate::error::AppError::ProcessError(e.to_string()))
}
