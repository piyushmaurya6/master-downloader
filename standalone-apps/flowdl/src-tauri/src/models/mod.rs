use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub name: String,
    pub installed: bool,
    pub functional: bool,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub path: Option<String>,
    pub source: String, // "managed" | "system" | "winget" | "none"
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyState {
    pub ytdlp: ToolStatus,
    pub ffmpeg: ToolStatus,
    pub ffprobe: ToolStatus,
    pub ready: bool,
    pub checking: bool,
    pub installing: bool,
    pub install_step: String,
    pub install_progress: f64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFormatOption {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub fps: Option<u32>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub filesize_approx: Option<u64>,
    pub tbr: Option<f64>,
    pub has_video: bool,
    pub has_audio: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleTrack {
    pub lang: String,
    pub name: String,
    pub is_auto_generated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntryPreview {
    pub id: String,
    pub title: String,
    pub duration: Option<f64>,
    pub url: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub id: String,
    pub title: String,
    pub url: String,
    pub webpage_url: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub channel_url: Option<String>,
    pub duration: Option<f64>,
    pub duration_string: String,
    pub extractor: String,
    pub upload_date: Option<String>,
    pub description: Option<String>,
    pub is_playlist: bool,
    pub playlist_title: Option<String>,
    pub playlist_count: Option<usize>,
    pub playlist_entries: Vec<PlaylistEntryPreview>,
    pub available_heights: Vec<u32>,
    pub available_video_codecs: Vec<String>,
    pub available_fps: Vec<u32>,
    pub available_audio_formats: Vec<VideoFormatOption>,
    pub subtitles: Vec<SubtitleTrack>,
    pub formats: Vec<VideoFormatOption>,
    pub estimated_sizes_by_height: HashMap<u32, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub extractor: String,
    pub mode: String, // "video" | "audio"
    pub video_quality: String,
    pub exact_resolution: bool,
    pub container: String, // "mp4" | "mkv" | "webm" | "auto"
    pub video_codec: String, // "auto" | "h264" | "h265" | "vp9" | "av1"
    pub preferred_fps: Option<u32>,
    pub audio_format: String, // "mp3" | "m4a" | "best" | "flac" | "wav" | "opus"
    pub audio_quality: String, // "best" | "320" | "256" | "192" | "128"
    pub download_subtitles: bool,
    pub subtitle_lang: String,
    pub embed_thumbnail: bool,
    pub embed_metadata: bool,
    pub download_chapters: bool,
    pub normalize_audio: bool,
    pub preserve_original_filename: bool,
    pub filename_template: Option<String>,
    pub speed_limit_kbps: Option<u32>,
    pub playlist_mode: String, // "single" | "playlist"
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub duration: Option<f64>,
    pub extractor: String,
    pub mode: String,
    pub format_summary: String,
    pub quality_label: String,
    pub status: String,
    pub postprocess_stage: Option<String>,
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_sec: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub output_path: Option<String>,
    pub output_dir: String,
    pub error_message: Option<String>,
    pub technical_error: Option<String>,
    pub error_category: Option<String>,
    #[serde(default)]
    pub recovery_action: Option<String>,
    #[serde(default)]
    pub suggest_engine_update: bool,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub request: DownloadRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub uploader: Option<String>,
    pub source_url: String,
    pub extractor: String,
    pub downloaded_at: String,
    pub mode: String,
    pub format_summary: String,
    pub quality: String,
    pub file_size: Option<u64>,
    pub output_path: Option<String>,
    pub output_dir: String,
    pub status: String,
    pub file_exists: bool,
    pub error_message: Option<String>,
    pub request: DownloadRequest,
}

fn default_ytdlp_channel() -> String {
    "stable".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub first_launch_completed: bool,
    pub prefer_managed_binaries: bool,
    pub ytdlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
    pub download_dir: String,
    pub default_video_container: String,
    pub default_video_quality: String,
    pub default_audio_format: String,
    pub default_audio_quality: String,
    pub theme: String,
    pub launch_on_startup: bool,
    pub minimize_to_tray: bool,
    pub max_concurrent_downloads: usize,
    pub filename_template: String,
    pub create_playlist_folders: bool,
    pub ask_before_overwrite: bool,
    pub download_subtitles_by_default: bool,
    pub default_subtitle_lang: String,
    pub preserve_intermediate_files: bool,
    pub speed_limit_kbps: u32,
    pub retry_count: u32,
    pub auto_update_engines: bool,
    pub clipboard_detection_enabled: bool,
    #[serde(default = "default_ytdlp_channel")]
    pub ytdlp_channel: String,
    #[serde(default = "default_true")]
    pub check_app_updates_on_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticLogEntry {
    pub timestamp: String,
    pub level: String,
    pub component: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEventPayload {
    pub download_id: String,
    pub state: String,
    pub progress: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bytes_per_sec: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub current_operation: Option<String>,
    pub output_path: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_date: Option<String>,
    pub release_notes: Option<String>,
    pub download_url: Option<String>,
    pub download_size: Option<u64>,
    pub signature_verified: bool,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineUpdateInfo {
    pub engine: String,
    pub current_version: Option<String>,
    pub latest_version: String,
    pub channel: String,
    pub update_available: bool,
    pub release_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub generated_at: String,
    pub app_version: String,
    pub os_version: String,
    pub arch: String,
    pub schema_version: u32,
    pub ytdlp_version: Option<String>,
    pub ffmpeg_version: Option<String>,
    pub ffprobe_version: Option<String>,
    pub deno_version: Option<String>,
    pub recent_logs: Vec<DiagnosticLogEntry>,
    pub markdown_report: String,
}
