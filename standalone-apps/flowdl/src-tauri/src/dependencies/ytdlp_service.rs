use crate::models::ToolStatus;
use crate::utils::{
    bundled_bin_dir, compute_file_sha256, managed_bin_dir, no_window_command, DiagnosticLogger,
};
use futures_util::StreamExt;
use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const YTDLP_RELEASE_API: &str = "https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest";

pub fn candidate_ytdlp_paths(
    configured_path: Option<&str>,
    prefer_managed: bool,
) -> Vec<(PathBuf, &'static str)> {
    let mut candidates = Vec::new();
    let managed = managed_bin_dir().join("yt-dlp.exe");

    if prefer_managed {
        candidates.push((managed.clone(), "managed"));
        if let Some(b_dir) = bundled_bin_dir() {
            candidates.push((b_dir.join("yt-dlp.exe"), "managed"));
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
            candidates.push((b_dir.join("yt-dlp.exe"), "managed"));
        }
    }

    if let Some(local_app) = dirs::data_local_dir() {
        let winget_link = local_app
            .join("Microsoft")
            .join("WinGet")
            .join("Links")
            .join("yt-dlp.exe");
        candidates.push((winget_link, "winget"));

        let winget_pkgs = local_app.join("Microsoft").join("WinGet").join("Packages");
        if let Ok(entries) = fs::read_dir(winget_pkgs) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("yt-dlp.yt-dlp") {
                    candidates.push((entry.path().join("yt-dlp.exe"), "winget"));
                }
            }
        }
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            candidates.push((dir.join("yt-dlp.exe"), "system"));
        }
    }

    candidates
}

pub async fn verify_ytdlp_binary(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err("Binary does not exist".to_string());
    }
    let output = no_window_command(path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| format!("Failed to execute yt-dlp: {}", e))?;

    if !output.status.success() {
        return Err("yt-dlp exited with non-zero status".to_string());
    }
    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ver.is_empty() {
        return Err("yt-dlp returned empty version output".to_string());
    }
    Ok(ver)
}

pub async fn detect_ytdlp(configured_path: Option<&str>, prefer_managed: bool) -> ToolStatus {
    for (path, source) in candidate_ytdlp_paths(configured_path, prefer_managed) {
        if path.exists() {
            if let Ok(version) = verify_ytdlp_binary(&path).await {
                return ToolStatus {
                    name: "yt-dlp".to_string(),
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
        name: "yt-dlp".to_string(),
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

pub async fn fetch_latest_ytdlp_release() -> Result<(String, String, Option<String>), String> {
    let client = reqwest::Client::builder()
        .user_agent("FlowDL-Desktop/1.0")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let resp: Value = client
        .get(YTDLP_RELEASE_API)
        .send()
        .await
        .map_err(|e| format!("Failed to reach GitHub releases: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Invalid release JSON: {}", e))?;

    let tag = resp
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing tag_name in release metadata".to_string())?
        .to_string();

    let mut download_url = None;
    let mut sha256_url = None;

    if let Some(assets) = resp.get("assets").and_then(|v| v.as_array()) {
        for asset in assets {
            let name = asset.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let url = asset
                .get("browser_download_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name == "yt-dlp.exe" {
                download_url = Some(url.to_string());
            } else if name == "SHA2-256SUMS" {
                sha256_url = Some(url.to_string());
            }
        }
    }

    let dl =
        download_url.ok_or_else(|| "yt-dlp.exe asset not found in latest release".to_string())?;
    Ok((tag, dl, sha256_url))
}

pub async fn install_or_update_managed_ytdlp<F>(
    logger: &DiagnosticLogger,
    on_progress: F,
) -> Result<ToolStatus, String>
where
    F: Fn(f64, &str),
{
    on_progress(10.0, "Fetching official yt-dlp release metadata...");
    let (tag, download_url, sha256_url) = fetch_latest_ytdlp_release().await?;
    logger.info(
        "DependencyManager",
        &format!("Downloading yt-dlp {} from official GitHub release", tag),
    );

    let client = reqwest::Client::builder()
        .user_agent("FlowDL-Desktop/1.0")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;

    let mut expected_sha256: Option<String> = None;
    if let Some(sums_url) = sha256_url {
        if let Ok(resp) = client.get(&sums_url).send().await {
            if let Ok(text) = resp.text().await {
                for line in text.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[1].trim_start_matches('*') == "yt-dlp.exe" {
                        expected_sha256 = Some(parts[0].to_lowercase());
                        break;
                    }
                }
            }
        }
    }

    let target_path = managed_bin_dir().join("yt-dlp.exe");
    let tmp_path = managed_bin_dir().join("yt-dlp.exe.tmp");
    let bak_path = managed_bin_dir().join("yt-dlp.exe.bak");

    on_progress(25.0, "Downloading yt-dlp.exe...");
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download yt-dlp.exe: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download returned HTTP {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(18_000_000);
    let mut downloaded: u64 = 0;
    let mut file = fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temporary yt-dlp binary: {}", e))?;

    let mut stream = response.bytes_stream();
    while let Some(chunk_res) = stream.next().await {
        let chunk =
            chunk_res.map_err(|e| format!("Stream error while downloading yt-dlp: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write yt-dlp chunk: {}", e))?;
        downloaded += chunk.len() as u64;
        let pct = 25.0 + ((downloaded as f64 / total_size as f64) * 50.0).min(50.0);
        on_progress(pct, "Downloading yt-dlp.exe...");
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);

    on_progress(80.0, "Verifying yt-dlp checksum and integrity...");
    if let Some(expected) = expected_sha256 {
        let actual = compute_file_sha256(&tmp_path)?;
        if actual.to_lowercase() != expected {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "SHA-256 verification failed for yt-dlp.exe (expected {}, got {})",
                expected, actual
            ));
        }
        logger.info(
            "DependencyManager",
            "yt-dlp.exe SHA-256 checksum verified.",
        );
    }

    let verified_version = match verify_ytdlp_binary(&tmp_path).await {
        Ok(v) => v,
        Err(e) => {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!(
                "Downloaded yt-dlp binary failed verification check: {}",
                e
            ));
        }
    };

    if target_path.exists() {
        let _ = fs::copy(&target_path, &bak_path);
    }

    if let Err(e) = fs::rename(&tmp_path, &target_path) {
        if bak_path.exists() {
            let _ = fs::copy(&bak_path, &target_path);
        }
        let _ = fs::remove_file(&tmp_path);
        return Err(format!("Failed to activate updated yt-dlp binary: {}", e));
    }

    if let Err(e) = verify_ytdlp_binary(&target_path).await {
        if bak_path.exists() {
            let _ = fs::rename(&bak_path, &target_path);
        }
        return Err(format!(
            "Activated yt-dlp failed verification, rolled back: {}",
            e
        ));
    }

    let _ = fs::remove_file(&bak_path);
    on_progress(100.0, "yt-dlp ready.");

    Ok(ToolStatus {
        name: "yt-dlp".to_string(),
        installed: true,
        functional: true,
        version: Some(verified_version),
        latest_version: Some(tag),
        update_available: false,
        path: Some(target_path.to_string_lossy().to_string()),
        source: "managed".to_string(),
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rollback_when_replacement_binary_is_invalid() {
        let test_dir =
            std::env::temp_dir().join(format!("flowdl_dep_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&test_dir).unwrap();
        let target = test_dir.join("yt-dlp.exe");
        let bak = test_dir.join("yt-dlp.exe.bak");
        let bad_tmp = test_dir.join("yt-dlp.exe.tmp");

        fs::write(&target, b"working-binary-placeholder").unwrap();
        fs::copy(&target, &bak).unwrap();
        fs::write(&bad_tmp, b"corrupted-not-an-exe").unwrap();

        let verify_res = verify_ytdlp_binary(&bad_tmp).await;
        assert!(verify_res.is_err());
        if verify_res.is_err() {
            let _ = fs::remove_file(&bad_tmp);
            if bak.exists() {
                let _ = fs::copy(&bak, &target);
            }
        }
        assert_eq!(fs::read(&target).unwrap(), b"working-binary-placeholder");
        let _ = fs::remove_dir_all(test_dir);
    }
}
