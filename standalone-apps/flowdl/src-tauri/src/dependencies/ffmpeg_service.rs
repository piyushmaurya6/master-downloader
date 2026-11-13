use crate::models::ToolStatus;
use crate::utils::{
    bundled_bin_dir, compute_file_sha256, managed_bin_dir, no_window_command, DiagnosticLogger,
};
use futures_util::StreamExt;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const FFMPEG_RELEASE_API: &str =
    "https://api.github.com/repos/yt-dlp/FFmpeg-Builds/releases/latest";

pub fn candidate_ffmpeg_paths(
    binary_name: &str,
    configured_path: Option<&str>,
    prefer_managed: bool,
) -> Vec<(PathBuf, &'static str)> {
    let mut candidates = Vec::new();
    let managed = managed_bin_dir().join(binary_name);

    if prefer_managed {
        candidates.push((managed.clone(), "managed"));
        if let Some(b_dir) = bundled_bin_dir() {
            candidates.push((b_dir.join(binary_name), "managed"));
        }
    }
    if let Some(cfg) = configured_path {
        if !cfg.trim().is_empty() {
            candidates.push((PathBuf::from(cfg), "managed"));
        }
    }
    if !prefer_managed {
        candidates.push((managed, "managed"));
        if let Some(b_dir) = bundled_bin_dir() {
            candidates.push((b_dir.join(binary_name), "managed"));
        }
    }

    if let Some(local_app) = dirs::data_local_dir() {
        let winget_link = local_app
            .join("Microsoft")
            .join("WinGet")
            .join("Links")
            .join(binary_name);
        candidates.push((winget_link, "winget"));

        let winget_pkgs = local_app.join("Microsoft").join("WinGet").join("Packages");
        if let Ok(entries) = fs::read_dir(winget_pkgs) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.to_lowercase().contains("ffmpeg") {
                    if let Ok(sub_entries) = fs::read_dir(entry.path()) {
                        for sub in sub_entries.flatten() {
                            let bin_candidate = sub.path().join("bin").join(binary_name);
                            candidates.push((bin_candidate, "winget"));
                        }
                    }
                }
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            candidates.push((dir.join(binary_name), "system"));
        }
    }

    candidates
}

pub async fn verify_ffmpeg_binary(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err("Binary does not exist".to_string());
    }
    let output = no_window_command(path)
        .arg("-version")
        .output()
        .await
        .map_err(|e| format!("Failed to execute {}: {}", path.display(), e))?;

    if !output.status.success() {
        return Err("Process exited with non-zero status".to_string());
    }
    let first_line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if first_line.is_empty() {
        return Err("Empty version output".to_string());
    }
    let concise = first_line
        .split_whitespace()
        .nth(2)
        .unwrap_or(&first_line)
        .to_string();
    Ok(concise)
}

pub async fn detect_ffmpeg_tool(
    tool_name: &str,
    binary_name: &str,
    configured_path: Option<&str>,
    prefer_managed: bool,
) -> ToolStatus {
    for (path, source) in candidate_ffmpeg_paths(binary_name, configured_path, prefer_managed) {
        if path.exists() {
            if let Ok(version) = verify_ffmpeg_binary(&path).await {
                return ToolStatus {
                    name: tool_name.to_string(),
                    installed: true,
                    functional: true,
                    version: Some(version),
                    latest_version: None,
                    update_available: false,
                    path: Some(path.to_string_lossy().to_string()),
                    source: source.to_string(),
                    error: None,
                };
            }
        }
    }

    ToolStatus {
        name: tool_name.to_string(),
        installed: false,
        functional: false,
        version: None,
        latest_version: None,
        update_available: false,
        path: None,
        source: "none".to_string(),
        error: None,
    }
}

pub async fn install_or_update_managed_ffmpeg<F>(
    logger: &DiagnosticLogger,
    on_progress: F,
) -> Result<(ToolStatus, ToolStatus), String>
where
    F: Fn(f64, &str),
{
    on_progress(10.0, "Fetching official FFmpeg release metadata...");
    let client = reqwest::Client::builder()
        .user_agent("FlowDL-Desktop/1.0")
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let resp: Value = client
        .get(FFMPEG_RELEASE_API)
        .send()
        .await
        .map_err(|e| format!("Failed to reach FFmpeg GitHub releases: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid FFmpeg release JSON: {}", e))?;

    let asset_name_target = "ffmpeg-master-latest-win64-gpl.zip";
    let mut download_url = None;
    let mut checksums_url = None;

    if let Some(assets) = resp.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            let name = asset.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name == asset_name_target {
                download_url = Some(url.to_string());
            } else if name == "checksums.sha256" {
                checksums_url = Some(url.to_string());
            }
        }
    }

    let dl_url =
        download_url.ok_or_else(|| "FFmpeg win64 zip asset not found in release".to_string())?;
    let mut expected_sha256: Option<String> = None;
    if let Some(cs_url) = checksums_url {
        if let Ok(r) = client.get(&cs_url).send().await {
            if let Ok(txt) = r.text().await {
                for line in txt.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1].trim_start_matches('*') == asset_name_target {
                        expected_sha256 = Some(parts[0].to_lowercase());
                        break;
                    }
                }
            }
        }
    }

    let zip_tmp = managed_bin_dir().join("ffmpeg-download.zip.tmp");
    on_progress(20.0, "Downloading FFmpeg & ffprobe package...");

    let response = client
        .get(&dl_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download FFmpeg archive: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("FFmpeg download returned HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(130_000_000);
    let mut downloaded: u64 = 0;
    let mut file = fs::File::create(&zip_tmp)
        .map_err(|e| format!("Failed to create temporary FFmpeg zip: {}", e))?;

    let mut stream = response.bytes_stream();
    while let Some(chunk_res) = stream.next().await {
        let chunk =
            chunk_res.map_err(|e| format!("Stream error while downloading FFmpeg: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write FFmpeg chunk: {}", e))?;
        downloaded += chunk.len() as u64;
        let pct = 20.0 + ((downloaded as f64 / total_size as f64) * 55.0).min(55.0);
        on_progress(pct, "Downloading FFmpeg & ffprobe...");
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    on_progress(78.0, "Verifying FFmpeg package checksum...");
    if let Some(expected) = expected_sha256 {
        let actual = compute_file_sha256(&zip_tmp)?;
        if actual.to_lowercase() != expected {
            let _ = fs::remove_file(&zip_tmp);
            return Err(format!(
                "SHA-256 verification failed for FFmpeg archive (expected {}, got {})",
                expected, actual
            ));
        }
        logger.info("DependencyManager", "FFmpeg archive SHA-256 verified.");
    }

    on_progress(85.0, "Extracting ffmpeg.exe and ffprobe.exe...");
    let bin_dir = managed_bin_dir();
    let zip_file = fs::File::open(&zip_tmp).map_err(|e| e.to_string())?;
    let mut archive =
        zip::ZipArchive::new(zip_file).map_err(|e| format!("Invalid zip archive: {}", e))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if name.ends_with("/bin/ffmpeg.exe") || name.ends_with("/bin/ffprobe.exe") {
            let exe_name = if name.ends_with("ffmpeg.exe") {
                "ffmpeg.exe"
            } else {
                "ffprobe.exe"
            };
            let dest_tmp = bin_dir.join(format!("{}.tmp", exe_name));
            let dest_final = bin_dir.join(exe_name);
            let mut out = fs::File::create(&dest_tmp).map_err(|e| e.to_string())?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            out.write_all(&buf).map_err(|e| e.to_string())?;
            drop(out);
            let _ = fs::rename(&dest_tmp, &dest_final);
        }
    }
    let _ = fs::remove_file(&zip_tmp);

    on_progress(95.0, "Verifying FFmpeg installation...");
    let ffmpeg_status = detect_ffmpeg_tool("ffmpeg", "ffmpeg.exe", None, true).await;
    let ffprobe_status = detect_ffmpeg_tool("ffprobe", "ffprobe.exe", None, true).await;

    if !ffmpeg_status.functional {
        return Err("Extracted ffmpeg.exe failed verification check.".to_string());
    }
    on_progress(100.0, "FFmpeg ready.");
    Ok((ffmpeg_status, ffprobe_status))
}
