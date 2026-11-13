use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

pub const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryInfo {
    pub name: String,
    pub executable: String,
    pub installed: bool,
    pub path: String,
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatusReport {
    pub bin_dir: String,
    pub all_ready: bool,
    pub yt_dlp: BinaryInfo,
    pub gallery_dl: BinaryInfo,
    pub ffmpeg: BinaryInfo,
    pub ffprobe: BinaryInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryBootstrapProgress {
    pub binary_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f64,
    pub status: String, // "checking" | "downloading" | "verifying" | "extracting" | "ready" | "error"
    pub sha256: Option<String>,
    pub version: Option<String>,
}

/// Thread-safe registry of active child process PIDs (`yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`).
/// Ensures orphaned processes are cleanly terminated on job cancel or `RunEvent::ExitRequested`.
#[derive(Clone, Default)]
pub struct ProcessRegistry {
    active_pids: Arc<Mutex<HashMap<String, Vec<u32>>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            active_pids: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register(&self, owner_id: &str, pid: u32) {
        if let Ok(mut map) = self.active_pids.lock() {
            map.entry(owner_id.to_string()).or_default().push(pid);
        }
    }

    pub fn unregister(&self, owner_id: &str, pid: u32) {
        if let Ok(mut map) = self.active_pids.lock() {
            if let Some(vec) = map.get_mut(owner_id) {
                vec.retain(|&p| p != pid);
                if vec.is_empty() {
                    map.remove(owner_id);
                }
            }
        }
    }

    pub fn kill_for_job(&self, job_id: &str) {
        let pids = if let Ok(mut map) = self.active_pids.lock() {
            map.remove(job_id).unwrap_or_default()
        } else {
            Vec::new()
        };
        for pid in pids {
            terminate_pid_tree(pid);
        }
    }

    pub fn kill_all(&self) {
        let all_pids: Vec<u32> = if let Ok(mut map) = self.active_pids.lock() {
            let collected = map.values().flatten().copied().collect();
            map.clear();
            collected
        } else {
            Vec::new()
        };
        for pid in all_pids {
            terminate_pid_tree(pid);
        }
    }
}

fn terminate_pid_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
}

/// Creates a `tokio::process::Command` guaranteed to run with `CREATE_NO_WINDOW` (`0x08000000`) on Windows
/// and `kill_on_drop(true)` so console windows never flash and child processes do not leak.
pub fn new_hidden_tokio_command(binary_path: &Path) -> Command {
    let mut cmd = Command::new(binary_path);
    cmd.kill_on_drop(true);
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[derive(Clone)]
pub struct BinaryManager {
    bin_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseResponse {
    tag_name: String,
    assets: Vec<GithubReleaseAsset>,
}

impl BinaryManager {
    pub fn new(bin_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&bin_dir);
        Self { bin_dir }
    }

    pub fn default_bin_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("InstaFlow")
            .join("bin")
    }

    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    pub fn yt_dlp_path(&self) -> PathBuf {
        self.bin_dir.join("yt-dlp.exe")
    }

    pub fn gallery_dl_path(&self) -> PathBuf {
        self.bin_dir.join("gallery-dl.exe")
    }

    pub fn ffmpeg_path(&self) -> PathBuf {
        self.bin_dir.join("ffmpeg.exe")
    }

    pub fn ffprobe_path(&self) -> PathBuf {
        self.bin_dir.join("ffprobe.exe")
    }

    /// Computes the hex-encoded SHA-256 digest of a file on disk.
    pub fn compute_file_sha256(path: &Path) -> AppResult<String> {
        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
        Ok(hex::encode(hasher.finalize()))
    }

    /// Inspects `%LOCALAPPDATA%\InstaFlow\bin\` and returns the readiness, version, and SHA-256 of each managed binary.
    pub async fn inspect_status(&self) -> BinaryStatusReport {
        let yt_dlp = self
            .inspect_single_binary("yt-dlp", "yt-dlp.exe", &["--version"])
            .await;
        let gallery_dl = self
            .inspect_single_binary("gallery-dl", "gallery-dl.exe", &["--version"])
            .await;
        let ffmpeg = self
            .inspect_single_binary("ffmpeg", "ffmpeg.exe", &["-version"])
            .await;
        let ffprobe = self
            .inspect_single_binary("ffprobe", "ffprobe.exe", &["-version"])
            .await;

        let all_ready =
            yt_dlp.installed && gallery_dl.installed && ffmpeg.installed && ffprobe.installed;

        BinaryStatusReport {
            bin_dir: self.bin_dir.to_string_lossy().to_string(),
            all_ready,
            yt_dlp,
            gallery_dl,
            ffmpeg,
            ffprobe,
        }
    }

    async fn inspect_single_binary(
        &self,
        name: &str,
        exe_name: &str,
        version_args: &[&str],
    ) -> BinaryInfo {
        let path = self.bin_dir.join(exe_name);
        if !path.exists() {
            return BinaryInfo {
                name: name.to_string(),
                executable: exe_name.to_string(),
                installed: false,
                path: path.to_string_lossy().to_string(),
                version: None,
                sha256: None,
                size_bytes: None,
            };
        }

        let size_bytes = std::fs::metadata(&path).ok().map(|m| m.len());
        let sha256 = Self::compute_file_sha256(&path).ok();
        let version = self.query_binary_version(&path, version_args).await;

        BinaryInfo {
            name: name.to_string(),
            executable: exe_name.to_string(),
            installed: size_bytes.unwrap_or(0) > 1024,
            path: path.to_string_lossy().to_string(),
            version,
            sha256,
            size_bytes,
        }
    }

    async fn query_binary_version(&self, path: &Path, args: &[&str]) -> Option<String> {
        let mut cmd = new_hidden_tokio_command(path);
        cmd.args(args);
        let output = tokio::time::timeout(Duration::from_secs(6), cmd.output())
            .await
            .ok()?
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()?.trim();
        if first_line.is_empty() {
            None
        } else {
            Some(first_line.chars().take(64).collect())
        }
    }

    /// Bootstraps or updates all required standalone Windows binaries (`yt-dlp.exe`, `gallery-dl.exe`, `ffmpeg.exe`, `ffprobe.exe`),
    /// streaming progress events (`binary-bootstrap-progress`) and verifying SHA-256 hashes before granting execution permission.
    pub async fn bootstrap_or_update_all(
        &self,
        app: Option<&AppHandle>,
        force_update: bool,
    ) -> AppResult<BinaryStatusReport> {
        std::fs::create_dir_all(&self.bin_dir)?;

        let client = reqwest::Client::builder()
            .user_agent("InstaFlow/1.0.0 (Windows; x64)")
            .timeout(Duration::from_secs(180))
            .build()?;

        // 1. Ensure yt-dlp.exe
        if force_update || !self.yt_dlp_path().exists() {
            self.download_and_verify_yt_dlp(&client, app).await?;
        } else {
            let sha = Self::compute_file_sha256(&self.yt_dlp_path()).ok();
            let ver = self
                .query_binary_version(&self.yt_dlp_path(), &["--version"])
                .await;
            Self::emit_progress(
                app,
                BinaryBootstrapProgress {
                    binary_name: "yt-dlp.exe".to_string(),
                    downloaded_bytes: 100,
                    total_bytes: Some(100),
                    percentage: 100.0,
                    status: "ready".to_string(),
                    sha256: sha,
                    version: ver,
                },
            );
        }

        // 2. Ensure gallery-dl.exe
        if force_update || !self.gallery_dl_path().exists() {
            self.download_and_verify_gallery_dl(&client, app).await?;
        } else {
            let sha = Self::compute_file_sha256(&self.gallery_dl_path()).ok();
            let ver = self
                .query_binary_version(&self.gallery_dl_path(), &["--version"])
                .await;
            Self::emit_progress(
                app,
                BinaryBootstrapProgress {
                    binary_name: "gallery-dl.exe".to_string(),
                    downloaded_bytes: 100,
                    total_bytes: Some(100),
                    percentage: 100.0,
                    status: "ready".to_string(),
                    sha256: sha,
                    version: ver,
                },
            );
        }

        // 3. Ensure ffmpeg.exe & ffprobe.exe
        if force_update || !self.ffmpeg_path().exists() || !self.ffprobe_path().exists() {
            self.download_and_verify_ffmpeg(&client, app).await?;
        } else {
            let sha = Self::compute_file_sha256(&self.ffmpeg_path()).ok();
            let ver = self
                .query_binary_version(&self.ffmpeg_path(), &["-version"])
                .await;
            Self::emit_progress(
                app,
                BinaryBootstrapProgress {
                    binary_name: "ffmpeg.exe".to_string(),
                    downloaded_bytes: 100,
                    total_bytes: Some(100),
                    percentage: 100.0,
                    status: "ready".to_string(),
                    sha256: sha,
                    version: ver,
                },
            );
        }

        Ok(self.inspect_status().await)
    }

    async fn download_and_verify_yt_dlp(
        &self,
        client: &reqwest::Client,
        app: Option<&AppHandle>,
    ) -> AppResult<()> {
        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: "yt-dlp.exe".to_string(),
                downloaded_bytes: 0,
                total_bytes: None,
                percentage: 0.0,
                status: "checking".to_string(),
                sha256: None,
                version: None,
            },
        );

        let release_meta = client
            .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
            .send()
            .await;

        let tag_version = if let Ok(resp) = release_meta {
            if let Ok(json) = resp.json::<GithubReleaseResponse>().await {
                Some(json.tag_name)
            } else {
                None
            }
        } else {
            None
        };

        let exe_url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
        let sums_url = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/SHA2-256SUMS";

        // Fetch official SHA2-256SUMS manifest
        let expected_hash = if let Ok(sums_resp) = client.get(sums_url).send().await {
            if let Ok(sums_text) = sums_resp.text().await {
                parse_sha256_manifest(&sums_text, "yt-dlp.exe")
            } else {
                None
            }
        } else {
            None
        };

        let tmp_path = self.bin_dir.join("yt-dlp.exe.part");
        let computed_hash = self
            .stream_download_to_file(
                client,
                exe_url,
                &tmp_path,
                "yt-dlp.exe",
                tag_version.clone(),
                app,
            )
            .await?;

        if let Some(ref exp) = expected_hash {
            if !exp.eq_ignore_ascii_case(&computed_hash) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(AppError::ChecksumMismatch);
            }
        }

        let final_path = self.yt_dlp_path();
        std::fs::rename(&tmp_path, &final_path)?;

        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: "yt-dlp.exe".to_string(),
                downloaded_bytes: 100,
                total_bytes: Some(100),
                percentage: 100.0,
                status: "ready".to_string(),
                sha256: Some(computed_hash),
                version: tag_version,
            },
        );
        Ok(())
    }

    async fn download_and_verify_gallery_dl(
        &self,
        client: &reqwest::Client,
        app: Option<&AppHandle>,
    ) -> AppResult<()> {
        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: "gallery-dl.exe".to_string(),
                downloaded_bytes: 0,
                total_bytes: None,
                percentage: 0.0,
                status: "checking".to_string(),
                sha256: None,
                version: None,
            },
        );

        let release_meta = client
            .get("https://api.github.com/repos/mikf/gallery-dl/releases/latest")
            .send()
            .await;

        let mut tag_version = Some("v1.32.19".to_string());
        let mut exe_url =
            "https://codeberg.org/mikf/gallery-dl/releases/download/v1.32.19/gallery-dl.exe"
                .to_string();

        if let Ok(resp) = release_meta {
            if let Ok(json) = resp.json::<GithubReleaseResponse>().await {
                tag_version = Some(json.tag_name);
                if let Some(asset) = json
                    .assets
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case("gallery-dl.exe"))
                {
                    exe_url = asset.browser_download_url.clone();
                }
            }
        }

        let sig_sums_url = format!("{}.sha256", exe_url);
        let expected_hash = if let Ok(sums_resp) = client.get(&sig_sums_url).send().await {
            if sums_resp.status().is_success() {
                if let Ok(sums_text) = sums_resp.text().await {
                    parse_sha256_manifest(&sums_text, "gallery-dl.exe")
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let tmp_path = self.bin_dir.join("gallery-dl.exe.part");
        let computed_hash = self
            .stream_download_to_file(
                client,
                &exe_url,
                &tmp_path,
                "gallery-dl.exe",
                tag_version.clone(),
                app,
            )
            .await?;

        if let Some(ref exp) = expected_hash {
            if !exp.eq_ignore_ascii_case(&computed_hash) {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(AppError::ChecksumMismatch);
            }
        }

        let final_path = self.gallery_dl_path();
        std::fs::rename(&tmp_path, &final_path)?;

        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: "gallery-dl.exe".to_string(),
                downloaded_bytes: 100,
                total_bytes: Some(100),
                percentage: 100.0,
                status: "ready".to_string(),
                sha256: Some(computed_hash),
                version: tag_version,
            },
        );
        Ok(())
    }

    async fn download_and_verify_ffmpeg(
        &self,
        client: &reqwest::Client,
        app: Option<&AppHandle>,
    ) -> AppResult<()> {
        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: "ffmpeg.exe".to_string(),
                downloaded_bytes: 0,
                total_bytes: None,
                percentage: 0.0,
                status: "checking".to_string(),
                sha256: None,
                version: None,
            },
        );

        let zip_name = "ffmpeg-master-latest-win64-gpl.zip";
        let zip_url = format!(
            "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/{}",
            zip_name
        );
        let sums_url =
            "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/checksums.sha256";

        let expected_hash = if let Ok(sums_resp) = client.get(sums_url).send().await {
            if let Ok(sums_text) = sums_resp.text().await {
                parse_sha256_manifest(&sums_text, zip_name)
            } else {
                None
            }
        } else {
            None
        };

        let tmp_zip = self.bin_dir.join("ffmpeg-win64.zip.part");
        let computed_hash = self
            .stream_download_to_file(
                client,
                &zip_url,
                &tmp_zip,
                "ffmpeg.exe",
                Some("latest-win64-gpl".to_string()),
                app,
            )
            .await?;

        if let Some(ref exp) = expected_hash {
            if !exp.eq_ignore_ascii_case(&computed_hash) {
                let _ = std::fs::remove_file(&tmp_zip);
                return Err(AppError::ChecksumMismatch);
            }
        }

        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: "ffmpeg.exe".to_string(),
                downloaded_bytes: 100,
                total_bytes: Some(100),
                percentage: 95.0,
                status: "extracting".to_string(),
                sha256: Some(computed_hash.clone()),
                version: Some("latest-win64-gpl".to_string()),
            },
        );

        // Extract ffmpeg.exe and ffprobe.exe from the verified archive
        let bin_dir_clone = self.bin_dir.clone();
        let tmp_zip_clone = tmp_zip.clone();
        tokio::task::spawn_blocking(move || -> AppResult<()> {
            let file = std::fs::File::open(&tmp_zip_clone)?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| AppError::ProcessError(format!("Invalid FFmpeg zip: {}", e)))?;

            for i in 0..archive.len() {
                let mut entry = archive
                    .by_index(i)
                    .map_err(|e| AppError::ProcessError(e.to_string()))?;
                let entry_name = entry.name().to_string();
                if entry_name.ends_with("/bin/ffmpeg.exe")
                    || entry_name.ends_with("/bin/ffprobe.exe")
                    || entry_name.ends_with(".dll")
                {
                    if let Some(fname) = Path::new(&entry_name).file_name() {
                        let out_path = bin_dir_clone.join(fname);
                        let mut out_file = std::fs::File::create(&out_path)?;
                        std::io::copy(&mut entry, &mut out_file)?;
                    }
                }
            }
            let _ = std::fs::remove_file(&tmp_zip_clone);
            Ok(())
        })
        .await
        .map_err(|e| AppError::ProcessError(e.to_string()))??;

        let ffmpeg_sha = Self::compute_file_sha256(&self.ffmpeg_path()).ok();
        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: "ffmpeg.exe".to_string(),
                downloaded_bytes: 100,
                total_bytes: Some(100),
                percentage: 100.0,
                status: "ready".to_string(),
                sha256: ffmpeg_sha,
                version: Some("latest-win64-gpl".to_string()),
            },
        );
        Ok(())
    }

    async fn stream_download_to_file(
        &self,
        client: &reqwest::Client,
        url: &str,
        dest: &Path,
        binary_name: &str,
        version: Option<String>,
        app: Option<&AppHandle>,
    ) -> AppResult<String> {
        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::ProcessError(format!(
                "Failed to download {} (HTTP {})",
                binary_name,
                resp.status()
            )));
        }

        let total_bytes = resp.content_length();
        let mut downloaded_bytes: u64 = 0;
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::create(dest)?;
        let mut stream = resp.bytes_stream();
        let mut last_emit = std::time::Instant::now();

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res?;
            file.write_all(&chunk)?;
            hasher.update(&chunk);
            downloaded_bytes += chunk.len() as u64;

            if last_emit.elapsed() >= Duration::from_millis(90) {
                let pct = total_bytes
                    .map(|tot| ((downloaded_bytes as f64 / tot as f64) * 100.0).clamp(0.0, 99.9))
                    .unwrap_or(50.0);

                Self::emit_progress(
                    app,
                    BinaryBootstrapProgress {
                        binary_name: binary_name.to_string(),
                        downloaded_bytes,
                        total_bytes,
                        percentage: pct,
                        status: "downloading".to_string(),
                        sha256: None,
                        version: version.clone(),
                    },
                );
                last_emit = std::time::Instant::now();
            }
        }

        file.flush()?;
        let final_hash = hex::encode(hasher.finalize());

        Self::emit_progress(
            app,
            BinaryBootstrapProgress {
                binary_name: binary_name.to_string(),
                downloaded_bytes,
                total_bytes: Some(downloaded_bytes),
                percentage: 99.9,
                status: "verifying".to_string(),
                sha256: Some(final_hash.clone()),
                version,
            },
        );

        Ok(final_hash)
    }

    fn emit_progress(app: Option<&AppHandle>, payload: BinaryBootstrapProgress) {
        if let Some(handle) = app {
            let _ = handle.emit("binary-bootstrap-progress", payload);
        }
    }
}

/// Parses a standard `SHA2-256SUMS` or `checksums.sha256` manifest file to extract the expected hex digest.
pub fn parse_sha256_manifest(manifest_text: &str, target_filename: &str) -> Option<String> {
    for line in manifest_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            let candidate_hash = parts[0];
            let candidate_file = parts[1].trim_start_matches('*');
            if candidate_file.eq_ignore_ascii_case(target_filename)
                && candidate_hash.len() == 64
                && candidate_hash.chars().all(|c| c.is_ascii_hexdigit())
            {
                return Some(candidate_hash.to_lowercase());
            }
        } else if parts.len() == 1 && parts[0].len() == 64 && parts[0].chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(parts[0].to_lowercase());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sha256_manifest() {
        let sample = r#"
a1b2c3d4e5f60123456789abcdef0123456789abcdef0123456789abcdef0123  yt-dlp
f9e8d7c6b5a40123456789abcdef0123456789abcdef0123456789abcdef9876 *yt-dlp.exe
        "#;
        let hash = parse_sha256_manifest(sample, "yt-dlp.exe").unwrap();
        assert_eq!(
            hash,
            "f9e8d7c6b5a40123456789abcdef0123456789abcdef0123456789abcdef9876"
        );
    }
}
