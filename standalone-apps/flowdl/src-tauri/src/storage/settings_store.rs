use crate::models::AppSettings;
use crate::utils::{app_data_dir, default_download_dir};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            first_launch_completed: false,
            prefer_managed_binaries: true,
            ytdlp_path: None,
            ffmpeg_path: None,
            ffprobe_path: None,
            download_dir: default_download_dir(),
            default_video_container: "mp4".to_string(),
            default_video_quality: "1080".to_string(),
            default_audio_format: "mp3".to_string(),
            default_audio_quality: "320".to_string(),
            theme: "dark".to_string(),
            launch_on_startup: false,
            minimize_to_tray: false,
            max_concurrent_downloads: 2,
            filename_template: "%(title)s [%(id)s].%(ext)s".to_string(),
            create_playlist_folders: true,
            ask_before_overwrite: false,
            download_subtitles_by_default: false,
            default_subtitle_lang: "en".to_string(),
            preserve_intermediate_files: false,
            speed_limit_kbps: 0,
            retry_count: 3,
            auto_update_engines: true,
            clipboard_detection_enabled: false,
            ytdlp_channel: "stable".to_string(),
            check_app_updates_on_startup: true,
        }
    }
}

#[derive(Clone)]
pub struct SettingsStore {
    file_path: PathBuf,
    settings: Arc<Mutex<AppSettings>>,
}

impl SettingsStore {
    pub fn new() -> Self {
        let file_path = app_data_dir().join("settings.json");
        let loaded = Self::load_from_path(&file_path).unwrap_or_default();
        Self {
            file_path,
            settings: Arc::new(Mutex::new(loaded)),
        }
    }

    pub fn with_path(file_path: PathBuf) -> Self {
        let loaded = Self::load_from_path(&file_path).unwrap_or_default();
        Self {
            file_path,
            settings: Arc::new(Mutex::new(loaded)),
        }
    }

    fn load_from_path(path: &PathBuf) -> Option<AppSettings> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn get(&self) -> AppSettings {
        self.settings
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn save(&self, updated: AppSettings) -> Result<AppSettings, String> {
        let mut sanitized = updated;
        if sanitized.max_concurrent_downloads == 0 {
            sanitized.max_concurrent_downloads = 1;
        } else if sanitized.max_concurrent_downloads > 6 {
            sanitized.max_concurrent_downloads = 6;
        }
        if sanitized.filename_template.trim().is_empty() {
            sanitized.filename_template = "%(title)s [%(id)s].%(ext)s".to_string();
        }

        if let Some(parent) = self.file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let json = serde_json::to_string_pretty(&sanitized)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        let tmp_path = self.file_path.with_extension("json.tmp");
        fs::write(&tmp_path, json).map_err(|e| format!("Failed to write settings file: {}", e))?;
        fs::rename(&tmp_path, &self.file_path)
            .map_err(|e| format!("Failed to finalize settings file: {}", e))?;

        if let Ok(mut guard) = self.settings.lock() {
            *guard = sanitized.clone();
        }
        Ok(sanitized)
    }
}
