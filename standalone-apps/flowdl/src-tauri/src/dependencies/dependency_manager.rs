use crate::dependencies::ffmpeg_service::{
    detect_ffmpeg_tool, install_or_update_managed_ffmpeg,
};
use crate::dependencies::ytdlp_service::{
    detect_ytdlp, fetch_latest_ytdlp_release, install_or_update_managed_ytdlp,
};
use crate::models::{DependencyState, ToolStatus};
use crate::storage::settings_store::SettingsStore;
use crate::utils::{ensure_bundled_binaries_seeded, managed_bin_dir, DiagnosticLogger};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter};

pub async fn check_all_dependencies(
    settings_store: &SettingsStore,
    check_remote_updates: bool,
    logger: &DiagnosticLogger,
) -> DependencyState {
    ensure_bundled_binaries_seeded(logger);
    let settings = settings_store.get();
    let mut ytdlp = detect_ytdlp(
        settings.ytdlp_path.as_deref(),
        settings.prefer_managed_binaries,
    )
    .await;

    let ffmpeg = detect_ffmpeg_tool(
        "ffmpeg",
        "ffmpeg.exe",
        settings.ffmpeg_path.as_deref(),
        settings.prefer_managed_binaries,
    )
    .await;

    let ffprobe = detect_ffmpeg_tool(
        "ffprobe",
        "ffprobe.exe",
        settings.ffprobe_path.as_deref(),
        settings.prefer_managed_binaries,
    )
    .await;

    if check_remote_updates {
        if let Ok((latest_tag, _, _)) = fetch_latest_ytdlp_release().await {
            ytdlp.latest_version = Some(latest_tag.clone());
            if let Some(ref current_ver) = ytdlp.version {
                ytdlp.update_available = current_ver.trim() != latest_tag.trim();
            }
        }
    }

    let ready = ytdlp.functional && ffmpeg.functional;
    logger.info(
        "DependencyManager",
        &format!(
            "Dependency check complete: yt-dlp={} ({:?}), ffmpeg={} ({:?})",
            ytdlp.functional, ytdlp.version, ffmpeg.functional, ffmpeg.version
        ),
    );

    DependencyState {
        ytdlp,
        ffmpeg,
        ffprobe,
        ready,
        checking: false,
        installing: false,
        install_step: if ready {
            "Ready to download".to_string()
        } else {
            "Missing required components".to_string()
        },
        install_progress: if ready { 100.0 } else { 0.0 },
        error: None,
    }
}

pub async fn setup_or_repair_dependencies(
    app: Option<&AppHandle>,
    settings_store: &SettingsStore,
    force_download: bool,
    active_downloads_count: usize,
    logger: &DiagnosticLogger,
) -> Result<DependencyState, String> {
    if active_downloads_count > 0 {
        return Err(
            "Cannot modify or update download engine binaries while a download is active."
                .to_string(),
        );
    }

    let emit_step = |progress: f64, step: &str| {
        if let Some(handle) = app {
            let _ = handle.emit(
                "dependency-setup-progress",
                serde_json::json!({
                    "progress": progress,
                    "step": step,
                }),
            );
        }
    };

    emit_step(5.0, "Checking system environment...");
    let current = check_all_dependencies(settings_store, false, logger).await;
    let bin_dir = managed_bin_dir();

    let mut ytdlp_status: ToolStatus;
    if !force_download && current.ytdlp.functional {
        if current.ytdlp.source != "managed" {
            if let Some(ref src_path) = current.ytdlp.path {
                let target = bin_dir.join("yt-dlp.exe");
                if fs::copy(Path::new(src_path), &target).is_ok() {
                    logger.info(
                        "DependencyManager",
                        "Adopted verified yt-dlp into FlowDL managed directory.",
                    );
                }
            }
        }
        ytdlp_status = detect_ytdlp(None, true).await;
    } else {
        emit_step(15.0, "Installing missing component: yt-dlp...");
        ytdlp_status = install_or_update_managed_ytdlp(logger, |pct, msg| {
            emit_step(15.0 + pct * 0.35, msg);
        })
        .await?;
    }

    let ffmpeg_status: ToolStatus;
    let ffprobe_status: ToolStatus;
    if !force_download && current.ffmpeg.functional {
        if current.ffmpeg.source != "managed" {
            if let Some(ref src_path) = current.ffmpeg.path {
                let src = Path::new(src_path);
                let target = bin_dir.join("ffmpeg.exe");
                let _ = fs::copy(src, &target);
                if let Some(parent) = src.parent() {
                    let probe_src = parent.join("ffprobe.exe");
                    if probe_src.exists() {
                        let _ = fs::copy(&probe_src, bin_dir.join("ffprobe.exe"));
                    }
                }
            }
        }
        ffmpeg_status = detect_ffmpeg_tool("ffmpeg", "ffmpeg.exe", None, true).await;
        ffprobe_status = detect_ffmpeg_tool("ffprobe", "ffprobe.exe", None, true).await;
    } else {
        emit_step(55.0, "Installing missing component: FFmpeg...");
        let (ff, fp) = install_or_update_managed_ffmpeg(logger, |pct, msg| {
            emit_step(55.0 + pct * 0.40, msg);
        })
        .await?;
        ffmpeg_status = ff;
        ffprobe_status = fp;
    }

    let mut settings = settings_store.get();
    settings.ytdlp_path = ytdlp_status.path.clone();
    settings.ffmpeg_path = ffmpeg_status.path.clone();
    settings.ffprobe_path = ffprobe_status.path.clone();
    settings.first_launch_completed = true;
    let _ = settings_store.save(settings);

    if let Ok((latest_tag, _, _)) = fetch_latest_ytdlp_release().await {
        ytdlp_status.latest_version = Some(latest_tag.clone());
        if let Some(ref v) = ytdlp_status.version {
            ytdlp_status.update_available = v.trim() != latest_tag.trim();
        }
    }

    let ready = ytdlp_status.functional && ffmpeg_status.functional;
    emit_step(100.0, "Ready to download");

    Ok(DependencyState {
        ytdlp: ytdlp_status,
        ffmpeg: ffmpeg_status,
        ffprobe: ffprobe_status,
        ready,
        checking: false,
        installing: false,
        install_step: "Ready to download".to_string(),
        install_progress: 100.0,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::no_window_command;

    #[tokio::test]
    async fn test_real_system_dependency_detection_and_ffmpeg_processing() {
        let temp_settings_path =
            std::env::temp_dir().join(format!("flowdl_settings_{}.json", uuid::Uuid::new_v4()));
        let store = SettingsStore::with_path(temp_settings_path.clone());
        let logger = DiagnosticLogger::new();

        let state = check_all_dependencies(&store, false, &logger).await;
        assert!(state.ytdlp.functional, "yt-dlp must be detected on system");
        assert!(state.ffmpeg.functional, "ffmpeg must be detected on system");
        assert!(state.ready, "DependencyState must be ready");

        // Verify real FFmpeg execution with Unicode & spaces path and no console window
        let ffmpeg_exe = state.ffmpeg.path.unwrap();
        let unicode_dir = std::env::temp_dir().join(format!(
            "FlowDL Real E2E 🎵 测试 {}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&unicode_dir).unwrap();

        let video_out = unicode_dir.join("Sample Video 1080p 🎵.mp4");
        let audio_out = unicode_dir.join("Sample Audio 320kbps 🎵.mp3");

        // Generate a real 1-second 1080p MP4 video + audio stream and merge with FFmpeg
        let v_status = no_window_command(&ffmpeg_exe)
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=1:size=1920x1080:rate=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:duration=1",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                video_out.to_str().unwrap(),
            ])
            .output()
            .await
            .unwrap();
        assert!(v_status.status.success(), "Real 1080p MP4 creation must succeed");
        assert!(video_out.exists());
        assert!(std::fs::metadata(&video_out).unwrap().len() > 1000);

        // Extract and convert audio to 320kbps MP3 from the MP4
        let a_status = no_window_command(&ffmpeg_exe)
            .args([
                "-y",
                "-i",
                video_out.to_str().unwrap(),
                "-vn",
                "-c:a",
                "libmp3lame",
                "-b:a",
                "320k",
                audio_out.to_str().unwrap(),
            ])
            .output()
            .await
            .unwrap();
        assert!(a_status.status.success(), "Real MP3 audio conversion must succeed");
        assert!(audio_out.exists());
        assert!(std::fs::metadata(&audio_out).unwrap().len() > 1000);

        let _ = std::fs::remove_dir_all(unicode_dir);
        let _ = std::fs::remove_file(temp_settings_path);
    }
}
