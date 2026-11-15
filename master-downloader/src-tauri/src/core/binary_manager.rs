use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

#[cfg(target_os = "windows")]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleBinaryStatus {
    pub name: String,
    pub installed: bool,
    pub path: String,
    pub version: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatusReport {
    pub all_ready: bool,
    pub bin_dir: String,
    pub yt_dlp: SingleBinaryStatus,
    pub gallery_dl: SingleBinaryStatus,
    pub ffmpeg: SingleBinaryStatus,
    pub ffprobe: SingleBinaryStatus,
    pub deno: SingleBinaryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryBootstrapProgress {
    pub binary_name: String,
    pub status: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f64,
}

#[derive(Clone, Default)]
pub struct ProcessRegistry {
    active_pids: Arc<Mutex<HashSet<u32>>>,
    job_pids: Arc<Mutex<HashMap<String, HashSet<u32>>>>,
    aborted_jobs: Arc<Mutex<HashSet<String>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            active_pids: Arc::new(Mutex::new(HashSet::new())),
            job_pids: Arc::new(Mutex::new(HashMap::new())),
            aborted_jobs: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn register_pid(&self, pid: u32) {
        if let Ok(mut guard) = self.active_pids.lock() {
            guard.insert(pid);
        }
    }

    pub fn register_job_pid(&self, job_id: &str, pid: u32) {
        self.register_pid(pid);
        if let Ok(mut map) = self.job_pids.lock() {
            map.entry(job_id.to_string()).or_default().insert(pid);
        }
    }

    pub fn unregister_pid(&self, pid: u32) {
        if let Ok(mut guard) = self.active_pids.lock() {
            guard.remove(&pid);
        }
    }

    pub fn unregister_job_pid(&self, job_id: &str, pid: u32) {
        self.unregister_pid(pid);
        if let Ok(mut map) = self.job_pids.lock() {
            if let Some(set) = map.get_mut(job_id) {
                set.remove(&pid);
                if set.is_empty() {
                    map.remove(job_id);
                }
            }
        }
    }

    pub fn clear_aborted(&self, job_id: &str) {
        if let Ok(mut aborted) = self.aborted_jobs.lock() {
            aborted.remove(job_id);
        }
    }

    pub fn clear_all_aborted(&self) {
        if let Ok(mut aborted) = self.aborted_jobs.lock() {
            aborted.clear();
        }
    }

    pub fn is_job_aborted(&self, job_id: &str) -> bool {
        self.aborted_jobs
            .lock()
            .map(|g| g.contains(job_id))
            .unwrap_or(false)
    }

    pub fn abort_and_kill_job(&self, job_id: &str) {
        if let Ok(mut aborted) = self.aborted_jobs.lock() {
            aborted.insert(job_id.to_string());
        }
        let pids: Vec<u32> = if let Ok(mut map) = self.job_pids.lock() {
            map.remove(job_id)
                .map(|s| s.into_iter().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        for pid in pids {
            self.kill_pid(pid);
        }
    }

    pub fn kill_pid(&self, pid: u32) {
        self.unregister_pid(pid);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
        }
    }

    pub fn kill_all(&self) {
        if let Ok(mut map) = self.job_pids.lock() {
            if let Ok(mut aborted) = self.aborted_jobs.lock() {
                for job_id in map.keys() {
                    aborted.insert(job_id.clone());
                }
            }
            map.clear();
        }

        let pids: Vec<u32> = if let Ok(mut guard) = self.active_pids.lock() {
            let list = guard.iter().copied().collect();
            guard.clear();
            list
        } else {
            Vec::new()
        };

        for pid in pids {
            self.kill_pid(pid);
        }
    }
}

#[derive(Clone)]
pub struct BinaryManager {
    pub bin_dir: PathBuf,
    pub bundled_bin_dir: Option<PathBuf>,
}

impl BinaryManager {
    pub fn new(app_data_dir: &Path, bundled_bin_dir: Option<PathBuf>) -> AppResult<Self> {
        let bin_dir = app_data_dir.join("bin");
        fs::create_dir_all(&bin_dir)?;
        let mgr = Self {
            bin_dir,
            bundled_bin_dir,
        };
        mgr.seed_bundled_binaries_on_startup();
        Ok(mgr)
    }

    pub fn seed_bundled_binaries_on_startup(&self) {
        let mut candidate_dirs: Vec<PathBuf> = Vec::new();
        if let Some(ref b) = self.bundled_bin_dir {
            candidate_dirs.push(b.clone());
        }
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                candidate_dirs.push(exe_dir.join("bin"));
                candidate_dirs.push(exe_dir.join("resources").join("bin"));
            }
        }
        candidate_dirs.push(PathBuf::from("resources/bin"));
        candidate_dirs.push(PathBuf::from("src-tauri/resources/bin"));

        for src_dir in candidate_dirs {
            if !src_dir.exists() {
                continue;
            }
            if let Ok(entries) = fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let src_path = entry.path();
                    if !src_path.is_file() {
                        continue;
                    }
                    let Some(file_name) = src_path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    let lower = file_name.to_lowercase();
                    if !(lower.ends_with(".exe") || lower.ends_with(".dll")) {
                        continue;
                    }
                    let dest_path = self.bin_dir.join(file_name);
                    if src_path == dest_path {
                        continue;
                    }
                    let needs_provision = match (fs::metadata(&src_path), fs::metadata(&dest_path)) {
                        (Ok(src_meta), Ok(dst_meta)) => {
                            dst_meta.len() < 10_000 || src_meta.len() != dst_meta.len()
                        }
                        (Ok(src_meta), Err(_)) => src_meta.len() >= 10_000,
                        _ => false,
                    };
                    if needs_provision {
                        if dest_path.exists() {
                            let _ = fs::remove_file(&dest_path);
                        }
                        // Prefer zero-copy NTFS hardlink on same drive (0 extra MB, <1ms); fallback to copy across drives
                        if fs::hard_link(&src_path, &dest_path).is_err() {
                            let _ = fs::copy(&src_path, &dest_path);
                        }
                    }
                }
            }
        }
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

    pub fn deno_path(&self) -> PathBuf {
        self.bin_dir.join("deno.exe")
    }

    pub async fn check_status(&self) -> BinaryStatusReport {
        self.seed_bundled_binaries_on_startup();

        let yt_dlp = self.inspect_single("yt-dlp.exe", &["--version"]).await;
        let gallery_dl = self.inspect_single("gallery-dl.exe", &["--version"]).await;
        let ffmpeg = self.inspect_single("ffmpeg.exe", &["-version"]).await;
        let ffprobe = self.inspect_single("ffprobe.exe", &["-version"]).await;
        let deno = self.inspect_single("deno.exe", &["--version"]).await;

        let all_ready = yt_dlp.installed
            && gallery_dl.installed
            && ffmpeg.installed
            && ffprobe.installed;

        BinaryStatusReport {
            all_ready,
            bin_dir: self.bin_dir.to_string_lossy().to_string(),
            yt_dlp,
            gallery_dl,
            ffmpeg,
            ffprobe,
            deno,
        }
    }

    async fn inspect_single(&self, filename: &str, version_args: &[&str]) -> SingleBinaryStatus {
        let path = self.bin_dir.join(filename);
        let path_str = path.to_string_lossy().to_string();

        let Ok(meta) = fs::metadata(&path) else {
            return SingleBinaryStatus {
                name: filename.to_string(),
                installed: false,
                path: path_str,
                version: None,
                sha256: None,
                size_bytes: None,
            };
        };

        if meta.len() < 10_000 {
            return SingleBinaryStatus {
                name: filename.to_string(),
                installed: false,
                path: path_str,
                version: None,
                sha256: None,
                size_bytes: Some(meta.len()),
            };
        }

        let sha256 = compute_file_sha256(&path).ok();
        let version = self.query_binary_version(&path, version_args).await;

        SingleBinaryStatus {
            name: filename.to_string(),
            installed: true,
            path: path_str,
            version,
            sha256,
            size_bytes: Some(meta.len()),
        }
    }

    async fn query_binary_version(&self, bin_path: &Path, args: &[&str]) -> Option<String> {
        let mut cmd = Command::new(bin_path);
        cmd.args(args);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = tokio::time::timeout(std::time::Duration::from_secs(6), cmd.output())
            .await
            .ok()?
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.lines().next().map(|l| l.trim().to_string())
    }

    pub async fn bootstrap_all(
        &self,
        app: Option<&AppHandle>,
        force_update: bool,
    ) -> AppResult<BinaryStatusReport> {
        self.seed_bundled_binaries_on_startup();

        let client = reqwest::Client::builder()
            .user_agent("MasterDownloader/1.0 (Windows NT 10.0; Win64; x64)")
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        if force_update || !self.is_valid_binary(&self.yt_dlp_path()) {
            let yt_url =
                "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe";
            let _ = self
                .download_with_backup(&client, "yt-dlp.exe", yt_url, app)
                .await;
        }

        if force_update || !self.is_valid_binary(&self.gallery_dl_path()) {
            let mut gdl_urls: Vec<String> = Vec::new();
            if let Ok(resp) = client
                .get("https://codeberg.org/api/v1/repos/mikf/gallery-dl/releases/latest")
                .send()
                .await
            {
                if let Ok(json) = resp.json::<serde_json::Value>().await {
                    if let Some(assets) = json.get("assets").and_then(|a| a.as_array()) {
                        for asset in assets {
                            let is_exe = asset
                                .get("name")
                                .and_then(|n| n.as_str())
                                .map(|n| n.eq_ignore_ascii_case("gallery-dl.exe"))
                                .unwrap_or(false);
                            if is_exe {
                                if let Some(u) =
                                    asset.get("browser_download_url").and_then(|u| u.as_str())
                                {
                                    gdl_urls.push(u.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            gdl_urls.push(
                "https://github.com/gdl-org/builds/releases/latest/download/gallery-dl_windows.exe"
                    .to_string(),
            );
            for url in &gdl_urls {
                if self
                    .download_with_backup(&client, "gallery-dl.exe", url, app)
                    .await
                    .is_ok()
                {
                    break;
                }
            }
        }

        Ok(self.check_status().await)
    }

    fn is_valid_binary(&self, path: &Path) -> bool {
        fs::metadata(path).map(|m| m.len() > 50_000).unwrap_or(false)
    }

    async fn download_with_backup(
        &self,
        client: &reqwest::Client,
        binary_name: &str,
        url: &str,
        app: Option<&AppHandle>,
    ) -> AppResult<()> {
        let target_path = self.bin_dir.join(binary_name);
        let temp_path = self.bin_dir.join(format!("{}.tmp", binary_name));
        let backup_path = self.bin_dir.join(format!("{}.bak", binary_name));

        emit_bootstrap(app, binary_name, "downloading", 0, None, 0.0);

        let resp = client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(AppError::Network(format!(
                "Failed to download {}: HTTP {}",
                binary_name,
                resp.status()
            )));
        }

        let total = resp.content_length();
        let mut file = File::create(&temp_path)?;
        let mut downloaded = 0u64;
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk?;
            file.write_all(&bytes)?;
            downloaded += bytes.len() as u64;
            let pct = total
                .map(|t| if t > 0 { (downloaded as f64 / t as f64) * 100.0 } else { 0.0 })
                .unwrap_or(50.0);
            emit_bootstrap(app, binary_name, "downloading", downloaded, total, pct);
        }
        file.flush()?;
        drop(file);

        if target_path.exists() {
            let _ = fs::copy(&target_path, &backup_path);
            let _ = fs::remove_file(&target_path);
        }
        fs::rename(&temp_path, &target_path)?;

        if self.query_binary_version(&target_path, &["--version"]).await.is_none() {
            if backup_path.exists() {
                let _ = fs::rename(&backup_path, &target_path);
            }
            return Err(AppError::ChecksumMismatch(format!(
                "{} failed post-update verification and was rolled back.",
                binary_name
            )));
        }

        emit_bootstrap(app, binary_name, "ready", downloaded, total, 100.0);
        Ok(())
    }
}

fn emit_bootstrap(
    app: Option<&AppHandle>,
    binary_name: &str,
    status: &str,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    percentage: f64,
) {
    if let Some(handle) = app {
        let _ = handle.emit(
            "binary-bootstrap-progress",
            BinaryBootstrapProgress {
                binary_name: binary_name.to_string(),
                status: status.to_string(),
                downloaded_bytes,
                total_bytes,
                percentage,
            },
        );
    }
}

pub fn compute_file_sha256(path: &Path) -> AppResult<String> {
    let mut file = File::open(path)?;
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
