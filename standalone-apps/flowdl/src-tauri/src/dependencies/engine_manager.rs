use crate::dependencies::dependency_manager::{
    check_all_dependencies, setup_or_repair_dependencies,
};
use crate::dependencies::ffmpeg_service::{
    detect_ffmpeg_tool, install_or_update_managed_ffmpeg,
};
use crate::dependencies::ytdlp_service::{
    detect_ytdlp, fetch_latest_ytdlp_release, install_or_update_managed_ytdlp, verify_ytdlp_binary,
};
use crate::models::{DependencyState, EngineUpdateInfo, ToolStatus};
use crate::storage::settings_store::SettingsStore;
use crate::utils::{bundled_bin_dir, ensure_bundled_binaries_seeded, managed_bin_dir, DiagnosticLogger};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// Independent EngineManager (System B — Section 4) for managing, verifying, updating,
/// rolling back, and self-repairing FlowDL's private `yt-dlp` and `FFmpeg` binaries.
pub struct EngineManager;

impl EngineManager {
    /// Detects installed and bundled engine binaries without modifying system-wide installations.
    pub async fn detect(
        settings_store: &SettingsStore,
        logger: &DiagnosticLogger,
    ) -> DependencyState {
        check_all_dependencies(settings_store, false, logger).await
    }

    /// Installs or seeds missing engine binaries into FlowDL's private `%LOCALAPPDATA%\FlowDL\bin` directory.
    pub async fn install(
        app: Option<&AppHandle>,
        settings_store: &SettingsStore,
        active_downloads_count: usize,
        logger: &DiagnosticLogger,
    ) -> Result<DependencyState, String> {
        setup_or_repair_dependencies(app, settings_store, false, active_downloads_count, logger)
            .await
    }

    /// Checks the exact version string reported by the active engine executable.
    pub async fn check_version(
        engine: &str,
        settings_store: &SettingsStore,
    ) -> Result<Option<String>, String> {
        let settings = settings_store.get();
        match engine {
            "yt-dlp" => {
                let st = detect_ytdlp(
                    settings.ytdlp_path.as_deref(),
                    settings.prefer_managed_binaries,
                )
                .await;
                Ok(st.version)
            }
            "ffmpeg" => {
                let st = detect_ffmpeg_tool(
                    "ffmpeg",
                    "ffmpeg.exe",
                    settings.ffmpeg_path.as_deref(),
                    settings.prefer_managed_binaries,
                )
                .await;
                Ok(st.version)
            }
            "ffprobe" => {
                let st = detect_ffmpeg_tool(
                    "ffprobe",
                    "ffprobe.exe",
                    settings.ffprobe_path.as_deref(),
                    settings.prefer_managed_binaries,
                )
                .await;
                Ok(st.version)
            }
            _ => Err(format!("Unknown engine: {}", engine)),
        }
    }

    /// Checks official release metadata for newer compatible `yt-dlp` versions and emits `engine:update-available`.
    pub async fn check_updates(
        app: Option<&AppHandle>,
        settings_store: &SettingsStore,
        logger: &DiagnosticLogger,
    ) -> Result<EngineUpdateInfo, String> {
        let settings = settings_store.get();
        let ytdlp = detect_ytdlp(
            settings.ytdlp_path.as_deref(),
            settings.prefer_managed_binaries,
        )
        .await;
        let (latest_tag, _, _) = fetch_latest_ytdlp_release().await?;
        let update_available = match &ytdlp.version {
            Some(cur) => cur.trim() != latest_tag.trim(),
            None => true,
        };

        let info = EngineUpdateInfo {
            engine: "yt-dlp".to_string(),
            current_version: ytdlp.version,
            latest_version: latest_tag,
            channel: settings.ytdlp_channel.clone(),
            update_available,
            release_date: None,
        };

        if info.update_available {
            logger.info(
                "EngineManager",
                &format!(
                    "Engine update available for yt-dlp: {} (current: {:?})",
                    info.latest_version, info.current_version
                ),
            );
            if let Some(handle) = app {
                let _ = handle.emit("engine:update-available", &info);
            }
        }

        Ok(info)
    }

    /// Downloads a verified release from the official upstream source, validates SHA-256 checksums,
    /// performs a compatibility health check, and activates the new version (with automatic rollback on failure).
    pub async fn download_verified_release(
        engine: &str,
        active_downloads_count: usize,
        logger: &DiagnosticLogger,
    ) -> Result<ToolStatus, String> {
        if active_downloads_count > 0 {
            return Err(format!(
                "Cannot update {} while {} download(s) are active.",
                engine, active_downloads_count
            ));
        }

        match engine {
            "yt-dlp" => install_or_update_managed_ytdlp(logger, |_, _| {}).await,
            "ffmpeg" => {
                let (ff, _) = install_or_update_managed_ffmpeg(logger, |_, _| {}).await?;
                Ok(ff)
            }
            _ => Err(format!("Unsupported engine for update: {}", engine)),
        }
    }

    /// Atomically activates a staged binary after verifying it starts and passes health checks,
    /// preserving the previous binary as `.bak` for rollback.
    pub async fn activate_version(
        staged_path: &Path,
        target_path: &Path,
        backup_path: &Path,
    ) -> Result<String, String> {
        let ver = verify_ytdlp_binary(staged_path).await?;
        if target_path.exists() {
            let _ = fs::copy(target_path, backup_path);
        }
        if let Err(e) = fs::rename(staged_path, target_path) {
            let _ = Self::rollback(target_path, backup_path);
            return Err(format!("Failed to activate staged binary: {}", e));
        }
        verify_ytdlp_binary(target_path).await.map_err(|e| {
            let _ = Self::rollback(target_path, backup_path);
            e
        })?;
        Ok(ver)
    }

    /// Restores the preserved `.bak` binary if an update activation or validation fails.
    pub fn rollback(target_path: &Path, backup_path: &Path) -> Result<(), String> {
        if backup_path.exists() {
            fs::copy(backup_path, target_path)
                .map_err(|e| format!("Failed to rollback binary from backup: {}", e))?;
            Ok(())
        } else {
            Err("No backup binary available for rollback.".to_string())
        }
    }

    /// Repairs a missing or broken engine installation by first restoring from the installer's bundled
    /// binaries if available, and falling back to a verified upstream download if needed.
    pub async fn repair(
        app: Option<&AppHandle>,
        settings_store: &SettingsStore,
        active_downloads_count: usize,
        logger: &DiagnosticLogger,
    ) -> Result<DependencyState, String> {
        if active_downloads_count > 0 {
            return Err(
                "Cannot repair download engine binaries while a download is active.".to_string(),
            );
        }

        // First attempt instant local restoration from bundled binaries if present
        if let Some(bundled) = bundled_bin_dir() {
            let target = managed_bin_dir();
            if let Ok(entries) = fs::read_dir(&bundled) {
                for entry in entries.flatten() {
                    let src = entry.path();
                    if src.is_file() {
                        let dest = target.join(entry.file_name());
                        let _ = fs::copy(&src, &dest);
                    }
                }
            }
        }
        ensure_bundled_binaries_seeded(logger);

        let state = check_all_dependencies(settings_store, false, logger).await;
        if state.ready {
            logger.info(
                "EngineManager",
                "Engine repair succeeded using verified bundled binaries.",
            );
            return Ok(state);
        }

        // Fallback to downloading fresh verified releases from official sources
        setup_or_repair_dependencies(app, settings_store, true, active_downloads_count, logger)
            .await
    }

    /// Performs a full health check of `yt-dlp`, `ffmpeg`, and `ffprobe`.
    pub async fn health_check(
        settings_store: &SettingsStore,
        logger: &DiagnosticLogger,
    ) -> DependencyState {
        check_all_dependencies(settings_store, false, logger).await
    }
}
