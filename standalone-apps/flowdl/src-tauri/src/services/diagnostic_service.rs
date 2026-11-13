use crate::dependencies::dependency_manager::check_all_dependencies;
use crate::models::DiagnosticReport;
use crate::platform::os_version_summary;
use crate::services::app_updater::CURRENT_APP_VERSION;
use crate::storage::history_store::CURRENT_SCHEMA_VERSION;
use crate::storage::settings_store::SettingsStore;
use crate::utils::{find_bundled_deno_path, no_window_command, sanitize_text_for_log, DiagnosticLogger};
use chrono::Utc;
use std::time::Duration;
use tokio::time::timeout;

async fn get_deno_version() -> Option<String> {
    let deno_path = find_bundled_deno_path()?;
    let out = timeout(
        Duration::from_secs(5),
        no_window_command(&deno_path).arg("--version").output(),
    )
    .await
    .ok()?
    .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .map(|s| s.trim().to_string())
}

pub async fn generate_sanitized_diagnostic_report(
    settings_store: &SettingsStore,
    logger: &DiagnosticLogger,
) -> DiagnosticReport {
    let deps = check_all_dependencies(settings_store, false, logger).await;
    let deno_ver = get_deno_version().await;
    let generated_at = Utc::now().to_rfc3339();
    let os_ver = os_version_summary();
    let arch = std::env::consts::ARCH.to_string();
    let recent_logs: Vec<_> = logger
        .get_entries()
        .into_iter()
        .take(50)
        .map(|mut e| {
            e.message = sanitize_text_for_log(&e.message);
            e
        })
        .collect();

    let mut md = String::new();
    md.push_str("### FlowDL Sanitized Diagnostic Report\n\n");
    md.push_str(&format!("- **Generated At**: `{}`\n", generated_at));
    md.push_str(&format!("- **FlowDL Version**: `v{}`\n", CURRENT_APP_VERSION));
    md.push_str(&format!("- **OS / Architecture**: `{}`\n", os_ver));
    md.push_str(&format!(
        "- **Database Schema Version**: `v{}`\n",
        CURRENT_SCHEMA_VERSION
    ));
    md.push_str(&format!(
        "- **yt-dlp**: `{}` (source: `{}`)\n",
        deps.ytdlp.version.as_deref().unwrap_or("Not detected"),
        deps.ytdlp.source
    ));
    md.push_str(&format!(
        "- **FFmpeg**: `{}` (source: `{}`)\n",
        deps.ffmpeg.version.as_deref().unwrap_or("Not detected"),
        deps.ffmpeg.source
    ));
    md.push_str(&format!(
        "- **ffprobe**: `{}` (source: `{}`)\n",
        deps.ffprobe.version.as_deref().unwrap_or("Not detected"),
        deps.ffprobe.source
    ));
    md.push_str(&format!(
        "- **Deno JS Runtime**: `{}`\n\n",
        deno_ver.as_deref().unwrap_or("Not detected")
    ));
    md.push_str("#### Recent Sanitized Logs\n```text\n");
    for entry in &recent_logs {
        md.push_str(&format!(
            "[{}] [{}] [{}] {}\n",
            entry.timestamp, entry.level, entry.component, entry.message
        ));
    }
    md.push_str("```\n");

    DiagnosticReport {
        generated_at,
        app_version: CURRENT_APP_VERSION.to_string(),
        os_version: os_ver,
        arch,
        schema_version: CURRENT_SCHEMA_VERSION,
        ytdlp_version: deps.ytdlp.version,
        ffmpeg_version: deps.ffmpeg.version,
        ffprobe_version: deps.ffprobe.version,
        deno_version: deno_ver,
        recent_logs,
        markdown_report: md,
    }
}
