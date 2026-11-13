use crate::models::DiagnosticLogEntry;
use chrono::Utc;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::process::Command;

#[cfg(target_os = "windows")]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn app_data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("FlowDL");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn managed_bin_dir() -> PathBuf {
    let dir = app_data_dir().join("bin");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// Locates the bundled `bin` directory packaged alongside `FlowDL.exe` by the installer.
pub fn bundled_bin_dir() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_dir = exe_path.parent()?;

    let direct_bin = exe_dir.join("bin");
    if direct_bin.is_dir() && direct_bin.join("yt-dlp.exe").exists() {
        return Some(direct_bin);
    }

    let resources_bin = exe_dir.join("resources").join("bin");
    if resources_bin.is_dir() && resources_bin.join("yt-dlp.exe").exists() {
        return Some(resources_bin);
    }

    // Development fallback: src-tauri/resources/bin
    let dev_bin = PathBuf::from("resources").join("bin");
    if dev_bin.is_dir() && dev_bin.join("yt-dlp.exe").exists() {
        return Some(dev_bin);
    }

    None
}

/// Ensures that bundled private binaries (yt-dlp.exe, ffmpeg.exe, ffprobe.exe, deno.exe, and shared DLLs)
/// are seeded into the per-user managed bin directory if missing or if restored from a fresh install.
pub fn ensure_bundled_binaries_seeded(logger: &DiagnosticLogger) {
    let target_dir = managed_bin_dir();
    let Some(source_dir) = bundled_bin_dir() else {
        return;
    };

    // If source_dir and target_dir are the exact same path (e.g., per-user install in %LOCALAPPDATA%\FlowDL\bin),
    // the bundled binaries are already in place.
    if same_directory(&source_dir, &target_dir) {
        logger.info(
            "DependencyManager",
            &format!(
                "Bundled binaries already active in managed directory: {}",
                target_dir.display()
            ),
        );
        return;
    }

    if let Ok(entries) = fs::read_dir(&source_dir) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            if src_path.is_file() {
                let file_name = entry.file_name();
                let dest_path = target_dir.join(&file_name);
                if !dest_path.exists() {
                    if fs::copy(&src_path, &dest_path).is_ok() {
                        logger.info(
                            "DependencyManager",
                            &format!(
                                "Seeded bundled binary {:?} into {}",
                                file_name,
                                target_dir.display()
                            ),
                        );
                    }
                }
            }
        }
    }
}

fn same_directory(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

pub fn find_bundled_deno_path() -> Option<PathBuf> {
    let managed = managed_bin_dir().join("deno.exe");
    if managed.exists() {
        return Some(managed);
    }
    if let Some(bundled) = bundled_bin_dir() {
        let candidate = bundled.join("deno.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn logs_dir() -> PathBuf {
    let dir = app_data_dir().join("logs");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn temp_dir() -> PathBuf {
    let dir = app_data_dir().join("tmp");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn default_download_dir() -> String {
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("C:\\Downloads"))
        .to_string_lossy()
        .to_string()
}

/// Spawns a background command configured on Windows so NO console window ever flashes,
/// with the private FlowDL bin directory prepended to the child process's PATH.
pub fn no_window_command<S: AsRef<std::ffi::OsStr>>(program: S) -> Command {
    #[allow(unused_mut)]
    let mut std_cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std_cmd.creation_flags(CREATE_NO_WINDOW);
    }

    // Prepend private managed & bundled bin directories to child process PATH without modifying system PATH
    let mut path_entries = Vec::new();
    path_entries.push(managed_bin_dir().to_string_lossy().to_string());
    if let Some(b_dir) = bundled_bin_dir() {
        path_entries.push(b_dir.to_string_lossy().to_string());
    }
    if let Ok(existing_path) = std::env::var("PATH") {
        path_entries.push(existing_path);
    }
    std_cmd.env("PATH", path_entries.join(";"));

    Command::from(std_cmd)
}

/// Redacts sensitive query parameters, cookies, authorization headers, and personal Windows usernames from logs.
pub fn sanitize_text_for_log(input: &str) -> String {
    let query_re = Regex::new(
        r"(?i)([?&](?:token|access_token|auth|key|signature|sig|session|password|cookie)=)([^&\s]+)",
    )
    .unwrap();
    let header_re = Regex::new(
        r"(?i)((?:authorization|cookie|set-cookie|x-auth-token)\s*[:=]\s*)([^\r\n;]+)",
    )
    .unwrap();
    let user_path_re = Regex::new(r"(?i)(C:\\Users\\)([^\\/:*?\x22<>|\r\n]+)(\\)").unwrap();

    let step1 = query_re.replace_all(input, "${1}[REDACTED]");
    let step2 = header_re.replace_all(&step1, "${1}[REDACTED]");
    user_path_re
        .replace_all(&step2, "${1}[USER]${3}")
        .to_string()
}

/// Computes SHA-256 hex digest of a file on disk.
pub fn compute_file_sha256(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("Failed to open file for SHA256: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read file for SHA256: {}", e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Clone)]
pub struct DiagnosticLogger {
    entries: Arc<Mutex<Vec<DiagnosticLogEntry>>>,
    log_file: PathBuf,
}

impl DiagnosticLogger {
    pub fn new() -> Self {
        let log_file = logs_dir().join("flowdl.log");
        Self {
            entries: Arc::new(Mutex::new(Vec::with_capacity(500))),
            log_file,
        }
    }

    pub fn log(&self, level: &str, component: &str, message: &str) {
        let sanitized = sanitize_text_for_log(message);
        let entry = DiagnosticLogEntry {
            timestamp: Utc::now().to_rfc3339(),
            level: level.to_string(),
            component: component.to_string(),
            message: sanitized.clone(),
        };

        if let Ok(mut guard) = self.entries.lock() {
            if guard.len() >= 500 {
                guard.remove(0);
            }
            guard.push(entry.clone());
        }

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
        {
            let _ = writeln!(
                file,
                "[{}] [{}] [{}] {}",
                entry.timestamp, entry.level, entry.component, entry.message
            );
        }
    }

    pub fn info(&self, component: &str, message: &str) {
        self.log("INFO", component, message);
    }

    pub fn warn(&self, component: &str, message: &str) {
        self.log("WARN", component, message);
    }

    pub fn error(&self, component: &str, message: &str) {
        self.log("ERROR", component, message);
    }

    pub fn get_entries(&self) -> Vec<DiagnosticLogEntry> {
        self.entries
            .lock()
            .map(|g| g.iter().rev().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.entries.lock() {
            guard.clear();
        }
        let _ = fs::write(&self.log_file, "");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_text_for_log() {
        let raw = "Analyzing https://example.com/video?id=123&token=secret_abc_999&format=mp4";
        let sanitized = sanitize_text_for_log(raw);
        assert!(sanitized.contains("token=[REDACTED]"));
        assert!(!sanitized.contains("secret_abc_999"));
        assert!(sanitized.contains("id=123"));
    }
}
