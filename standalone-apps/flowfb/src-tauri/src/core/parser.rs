use crate::core::url_normalizer::NormalizedFacebookUrl;
use crate::error::{AppError, AppResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Inspecting,
    DownloadingVideo,
    DownloadingAudio,
    Merging,
    Completed,
    Failed,
    Paused,
    RateLimited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub job_id: String,
    pub item_index: usize,
    pub total_items: usize,
    pub status: JobStatus,
    pub stage_label: String,
    pub percentage: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bps: Option<u64>,
    pub eta_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarouselSlideItem {
    pub index: usize,       // 1-based item index
    pub media_type: String, // "image" | "video"
    pub width: u32,
    pub height: u32,
    pub display_url: String,
    pub video_url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub ext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormatOption {
    pub quality_key: String,      // e.g. "best", "1080p", "720p", "480p", "audio_mp3", "audio_m4a"
    pub format_id: String,        // yt-dlp format selector or format_id
    pub resolution_label: String, // e.g. "1080p Full HD", "720p HD", "Audio MP3 (320kbps)"
    pub badge: String,            // e.g. "DASH 1080P", "HD", "SD", "MP3 320K", "M4A AAC"
    pub vcodec: String,           // e.g. "H.264", "VP9", "AV1", "none"
    pub acodec: String,           // e.g. "AAC", "MP3", "Opus"
    pub ext: String,              // "mp4" | "mp3" | "m4a"
    pub height: Option<u32>,
    pub fps: Option<u32>,
    pub filesize_bytes: Option<u64>,
    pub is_dash: bool,
    pub is_audio_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInspectionResult {
    pub canonical_url: String,
    pub shortcode: Option<String>,
    pub url_type: String, // "reel" | "watch" | "video" | "share" | "post" | "photo" | "story" | "carousel"
    pub type_badge: String,
    pub owner_username: String,
    pub owner_full_name: Option<String>,
    pub title: String,
    pub caption: String,
    pub timestamp: Option<i64>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub view_count: Option<u64>,
    pub like_count: Option<u64>,
    pub aspect_ratio_label: String, // "9:16" | "16:9" | "1:1" | "4:5"
    pub total_items: usize,
    pub slides: Vec<CarouselSlideItem>,
    pub available_qualities: Vec<String>,
    pub format_options: Vec<VideoFormatOption>,
}

/// Throttles `download-progress` IPC event emissions to at most once every `100ms` per job.
pub struct ThrottledProgressEmitter {
    last_emit: Option<Instant>,
    min_interval: Duration,
}

impl ThrottledProgressEmitter {
    pub fn new() -> Self {
        Self {
            last_emit: None,
            min_interval: Duration::from_millis(100),
        }
    }

    pub fn emit_if_ready(
        &mut self,
        app: Option<&AppHandle>,
        payload: &ProgressPayload,
        force: bool,
    ) {
        let now = Instant::now();
        let should_emit = force
            || match self.last_emit {
                Some(prev) => now.duration_since(prev) >= self.min_interval,
                None => true,
            };

        if should_emit {
            self.last_emit = Some(now);
            if let Some(handle) = app {
                let _ = handle.emit("download-progress", payload);
            }
        }
    }
}

/// Parses a single `yt-dlp` stdout line into a `ProgressPayload` if it contains progress or FFmpeg muxing telemetry.
pub fn parse_ytdlp_stdout_line(
    line: &str,
    job_id: &str,
    item_index: usize,
    total_items: usize,
    current_stream_stage: &mut JobStatus,
) -> Option<ProgressPayload> {
    static PROGRESS_RE: OnceLock<Regex> = OnceLock::new();
    let re = PROGRESS_RE.get_or_init(|| {
        Regex::new(
            r"\[download\]\s+(\d+(?:\.\d+)?)%\s+of\s+~?\s*(\d+(?:\.\d+)?)([KMGTP]?i?B)\s+at\s+(?:(\d+(?:\.\d+)?)([KMGTP]?i?B)/s|Unknown[^\s]*)\s+ETA\s+(?:(\d{2,}):(\d{2})(?::(\d{2}))?|Unknown)",
        )
        .unwrap()
    });

    let trimmed = line.trim();

    if trimmed.starts_with("[download] Destination:") {
        let lower = trimmed.to_lowercase();
        if lower.ends_with(".m4a")
            || lower.ends_with(".mp3")
            || lower.contains("audio")
            || lower.contains(".f140.")
        {
            *current_stream_stage = JobStatus::DownloadingAudio;
        } else {
            *current_stream_stage = JobStatus::DownloadingVideo;
        }
    }

    if trimmed.starts_with("[Merger]")
        || trimmed.starts_with("[ExtractAudio]")
        || trimmed.contains("Merging formats into")
    {
        *current_stream_stage = JobStatus::Merging;
        return Some(ProgressPayload {
            job_id: job_id.to_string(),
            item_index,
            total_items,
            status: JobStatus::Merging,
            stage_label: "Muxing lossless video + audio via FFmpeg...".to_string(),
            percentage: 99.0,
            downloaded_bytes: 0,
            total_bytes: None,
            speed_bps: None,
            eta_seconds: Some(1),
        });
    }

    if let Some(caps) = re.captures(trimmed) {
        let pct = caps
            .get(1)
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .unwrap_or(0.0)
            .clamp(0.0, 100.0);

        let total_val = caps
            .get(2)
            .and_then(|m| m.as_str().parse::<f64>().ok())
            .unwrap_or(0.0);
        let total_unit = caps.get(3).map(|m| m.as_str()).unwrap_or("B");
        let total_bytes = unit_to_bytes(total_val, total_unit);
        let downloaded_bytes = ((total_bytes as f64) * (pct / 100.0)).round() as u64;

        let speed_bps = match (caps.get(4), caps.get(5)) {
            (Some(v), Some(u)) => v
                .as_str()
                .parse::<f64>()
                .ok()
                .map(|val| unit_to_bytes(val, u.as_str())),
            _ => None,
        };

        let eta_seconds = match (caps.get(6), caps.get(7), caps.get(8)) {
            (Some(h), Some(m), Some(s)) => {
                let hh = h.as_str().parse::<u64>().unwrap_or(0);
                let mm = m.as_str().parse::<u64>().unwrap_or(0);
                let ss = s.as_str().parse::<u64>().unwrap_or(0);
                Some(hh * 3600 + mm * 60 + ss)
            }
            (Some(m), Some(s), None) => {
                let mm = m.as_str().parse::<u64>().unwrap_or(0);
                let ss = s.as_str().parse::<u64>().unwrap_or(0);
                Some(mm * 60 + ss)
            }
            _ => None,
        };

        let stage_label = if total_items > 1 {
            format!("Downloading item {} of {}...", item_index, total_items)
        } else if *current_stream_stage == JobStatus::DownloadingAudio {
            "Downloading DASH audio track...".to_string()
        } else {
            "Downloading DASH video stream...".to_string()
        };

        return Some(ProgressPayload {
            job_id: job_id.to_string(),
            item_index,
            total_items,
            status: current_stream_stage.clone(),
            stage_label,
            percentage: pct,
            downloaded_bytes,
            total_bytes: Some(total_bytes),
            speed_bps,
            eta_seconds,
        });
    }

    None
}

fn unit_to_bytes(val: f64, unit: &str) -> u64 {
    let multiplier: f64 = match unit {
        "KiB" | "KB" => 1024.0,
        "MiB" | "MB" => 1024.0 * 1024.0,
        "GiB" | "GB" => 1024.0 * 1024.0 * 1024.0,
        "TiB" | "TB" => 1024.0 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    (val * multiplier).round() as u64
}

/// Checks if a stderr line indicates a Facebook rate-limit, checkpoint, or login wall that should trip the circuit breaker.
pub fn is_circuit_breaker_trigger(stderr_line: &str) -> Option<String> {
    let lower = stderr_line.to_lowercase();
    if lower.contains("http error 429")
        || lower.contains("too many requests")
        || lower.contains("checkpoint")
        || lower.contains("temporarily blocked")
        || lower.contains("you're temporarily blocked")
        || lower.contains("please wait a few minutes")
    {
        return Some(stderr_line.trim().to_string());
    }
    None
}

fn friendly_vcodec_name(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.starts_with("avc") || lower.contains("h264") {
        "H.264".to_string()
    } else if lower.starts_with("vp9") || lower.starts_with("vp09") {
        "VP9".to_string()
    } else if lower.starts_with("av01") || lower.starts_with("av1") {
        "AV1".to_string()
    } else if lower.starts_with("hev") || lower.starts_with("hvc") || lower.contains("h265") {
        "HEVC".to_string()
    } else if lower == "none" {
        "none".to_string()
    } else {
        "H.264".to_string()
    }
}

fn compute_aspect_ratio_label(width: u32, height: u32, is_reel_hint: bool) -> String {
    if width == 0 || height == 0 {
        return if is_reel_hint {
            "9:16".to_string()
        } else {
            "16:9".to_string()
        };
    }
    let ratio = width as f64 / height as f64;
    if ratio < 0.68 {
        "9:16".to_string()
    } else if ratio < 0.90 {
        "4:5".to_string()
    } else if ratio <= 1.12 {
        "1:1".to_string()
    } else {
        "16:9".to_string()
    }
}

/// Extracts Facebook DASH + progressive format matrix from `yt-dlp` JSON `formats` array.
fn extract_facebook_format_matrix(
    v: &Value,
    duration_seconds: Option<f64>,
) -> Vec<VideoFormatOption> {
    let mut best_audio_bytes: Option<u64> = None;
    let mut height_buckets: HashMap<u32, VideoFormatOption> = HashMap::new();

    if let Some(formats) = v.get("formats").and_then(|f| f.as_array()) {
        // First pass: find best audio stream size for DASH muxing estimation
        for fmt in formats {
            let vcodec = fmt
                .get("vcodec")
                .and_then(|x| x.as_str())
                .unwrap_or("none");
            let acodec = fmt
                .get("acodec")
                .and_then(|x| x.as_str())
                .unwrap_or("none");
            if vcodec == "none" && acodec != "none" {
                let sz = fmt
                    .get("filesize")
                    .or_else(|| fmt.get("filesize_approx"))
                    .and_then(|x| x.as_u64())
                    .or_else(|| {
                        let abr = fmt.get("abr").or_else(|| fmt.get("tbr")).and_then(|x| x.as_f64())?;
                        let dur = duration_seconds?;
                        Some(((abr * 1000.0 / 8.0) * dur) as u64)
                    });
                if let Some(s) = sz {
                    if best_audio_bytes.map(|b| s > b).unwrap_or(true) {
                        best_audio_bytes = Some(s);
                    }
                }
            }
        }

        // Second pass: collect video formats by height tier
        for fmt in formats {
            let vcodec_raw = fmt
                .get("vcodec")
                .and_then(|x| x.as_str())
                .unwrap_or("none");
            let acodec_raw = fmt
                .get("acodec")
                .and_then(|x| x.as_str())
                .unwrap_or("none");
            let format_id = fmt
                .get("format_id")
                .and_then(|x| x.as_str())
                .unwrap_or("best");

            if vcodec_raw == "none" && format_id != "hd" && format_id != "sd" {
                continue;
            }

            let w_opt = fmt.get("width").and_then(|x| x.as_u64()).map(|w| w as u32);
            let h_opt = fmt.get("height").and_then(|x| x.as_u64()).map(|h| h as u32);
            let raw_height = match (w_opt, h_opt) {
                (Some(w), Some(h)) if w > 0 && h > 0 => Some(w.min(h)),
                (_, Some(h)) if h > 0 => Some(h),
                (Some(w), _) if w > 0 => Some(w),
                _ => {
                    if format_id.eq_ignore_ascii_case("hd") {
                        Some(720)
                    } else if format_id.eq_ignore_ascii_case("sd") {
                        Some(480)
                    } else {
                        None
                    }
                }
            };

            let Some(h) = raw_height else {
                continue;
            };

            let fps = fmt
                .get("fps")
                .and_then(|x| x.as_f64())
                .map(|f| f.round() as u32);
            let is_dash = acodec_raw == "none";
            let vcodec = friendly_vcodec_name(vcodec_raw);

            let video_size = fmt
                .get("filesize")
                .or_else(|| fmt.get("filesize_approx"))
                .and_then(|x| x.as_u64())
                .or_else(|| {
                    let tbr = fmt.get("vbr").or_else(|| fmt.get("tbr")).and_then(|x| x.as_f64())?;
                    let dur = duration_seconds?;
                    Some(((tbr * 1000.0 / 8.0) * dur) as u64)
                });

            let combined_size = match (video_size, is_dash, best_audio_bytes) {
                (Some(vs), true, Some(asize)) => Some(vs + asize),
                (Some(vs), _, _) => Some(vs),
                _ => None,
            };

            // Normalize height into standard tiers (2160, 1440, 1080, 720, 480, 360)
            let tier_height = if h >= 1900 {
                2160
            } else if h >= 1300 {
                1440
            } else if h >= 960 {
                1080
            } else if h >= 640 {
                720
            } else if h >= 420 {
                480
            } else {
                360
            };

            let (quality_key, resolution_label, badge) = match tier_height {
                2160 => (
                    "4k".to_string(),
                    "2160p 4K Ultra HD".to_string(),
                    "DASH 4K".to_string(),
                ),
                1440 => (
                    "1440p".to_string(),
                    "1440p QHD".to_string(),
                    "DASH 2K".to_string(),
                ),
                1080 => {
                    if fps.unwrap_or(30) >= 50 {
                        (
                            "1080p".to_string(),
                            "1080p60 Full HD".to_string(),
                            "DASH 1080P60".to_string(),
                        )
                    } else {
                        (
                            "1080p".to_string(),
                            "1080p Full HD".to_string(),
                            "DASH 1080P".to_string(),
                        )
                    }
                }
                720 => (
                    "720p".to_string(),
                    "720p HD".to_string(),
                    if is_dash {
                        "DASH 720P".to_string()
                    } else {
                        "HD 720P".to_string()
                    },
                ),
                480 => (
                    "480p".to_string(),
                    "480p SD".to_string(),
                    "SD 480P".to_string(),
                ),
                _ => (
                    "360p".to_string(),
                    "360p Fast SD".to_string(),
                    "SD 360P".to_string(),
                ),
            };

            let candidate = VideoFormatOption {
                quality_key,
                format_id: format_id.to_string(),
                resolution_label,
                badge,
                vcodec,
                acodec: "AAC".to_string(),
                ext: "mp4".to_string(),
                height: Some(tier_height),
                fps,
                filesize_bytes: combined_size,
                is_dash,
                is_audio_only: false,
            };

            let replace = match height_buckets.get(&tier_height) {
                Some(existing) => {
                    // Prefer H.264 or larger bitrate/filesize within the same resolution tier
                    let new_sz = candidate.filesize_bytes.unwrap_or(0);
                    let old_sz = existing.filesize_bytes.unwrap_or(0);
                    new_sz > old_sz
                }
                None => true,
            };

            if replace {
                height_buckets.insert(tier_height, candidate);
            }
        }
    }

    let mut options: Vec<VideoFormatOption> = Vec::new();

    // Always place "Best Available (Auto DASH + Audio)" first
    let max_tier = height_buckets.keys().copied().max().unwrap_or(1080);
    let max_size = height_buckets
        .get(&max_tier)
        .and_then(|o| o.filesize_bytes);

    options.push(VideoFormatOption {
        quality_key: "best".to_string(),
        format_id: "bestvideo+bestaudio/best".to_string(),
        resolution_label: format!("Best Available (Up to {}p)", max_tier),
        badge: "AUTO MAX".to_string(),
        vcodec: "H.264 / VP9".to_string(),
        acodec: "AAC".to_string(),
        ext: "mp4".to_string(),
        height: Some(max_tier),
        fps: None,
        filesize_bytes: max_size,
        is_dash: true,
        is_audio_only: false,
    });

    let mut sorted_tiers: Vec<u32> = height_buckets.keys().copied().collect();
    sorted_tiers.sort_by(|a, b| b.cmp(a));
    for tier in sorted_tiers {
        if let Some(opt) = height_buckets.remove(&tier) {
            options.push(opt);
        }
    }

    // Ensure standard 1080p and 720p presets exist even if Facebook only returned progressive metadata
    if !options.iter().any(|o| o.quality_key == "1080p") {
        options.push(VideoFormatOption {
            quality_key: "1080p".to_string(),
            format_id: "bestvideo[height<=1080]+bestaudio/best[height<=1080]/best".to_string(),
            resolution_label: "1080p Full HD".to_string(),
            badge: "FULL HD".to_string(),
            vcodec: "H.264".to_string(),
            acodec: "AAC".to_string(),
            ext: "mp4".to_string(),
            height: Some(1080),
            fps: Some(30),
            filesize_bytes: None,
            is_dash: true,
            is_audio_only: false,
        });
    }
    if !options.iter().any(|o| o.quality_key == "720p") {
        options.push(VideoFormatOption {
            quality_key: "720p".to_string(),
            format_id: "bestvideo[height<=720]+bestaudio/hd/best".to_string(),
            resolution_label: "720p HD".to_string(),
            badge: "HD 720P".to_string(),
            vcodec: "H.264".to_string(),
            acodec: "AAC".to_string(),
            ext: "mp4".to_string(),
            height: Some(720),
            fps: Some(30),
            filesize_bytes: None,
            is_dash: false,
            is_audio_only: false,
        });
    }

    // Append Audio Extraction Options (MP3 320kbps & M4A AAC)
    options.push(VideoFormatOption {
        quality_key: "audio_mp3".to_string(),
        format_id: "bestaudio/best".to_string(),
        resolution_label: "Audio Only — MP3 (320kbps)".to_string(),
        badge: "MP3 320K".to_string(),
        vcodec: "none".to_string(),
        acodec: "MP3".to_string(),
        ext: "mp3".to_string(),
        height: None,
        fps: None,
        filesize_bytes: best_audio_bytes.or_else(|| {
            duration_seconds.map(|d| ((320_000.0 / 8.0) * d) as u64)
        }),
        is_dash: false,
        is_audio_only: true,
    });

    options.push(VideoFormatOption {
        quality_key: "audio_m4a".to_string(),
        format_id: "bestaudio[ext=m4a]/bestaudio/best".to_string(),
        resolution_label: "Audio Only — M4A (Lossless AAC)".to_string(),
        badge: "M4A AAC".to_string(),
        vcodec: "none".to_string(),
        acodec: "AAC".to_string(),
        ext: "m4a".to_string(),
        height: None,
        fps: None,
        filesize_bytes: best_audio_bytes,
        is_dash: false,
        is_audio_only: true,
    });

    options
}

/// Parses `yt-dlp --dump-single-json` output for a Facebook Video, Reel, Watch stream, or Post into `MediaInspectionResult`.
pub fn parse_ytdlp_inspect_json(
    raw_json: &str,
    norm: &NormalizedFacebookUrl,
) -> AppResult<MediaInspectionResult> {
    let v: Value = serde_json::from_str(raw_json)
        .map_err(|e| AppError::EngineParseError(format!("Invalid yt-dlp JSON: {}", e)))?;

    let owner_username = v
        .get("uploader_id")
        .or_else(|| v.get("channel"))
        .or_else(|| v.get("uploader"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().trim_start_matches('@').to_string())
        .or_else(|| norm.username_hint.clone())
        .unwrap_or_else(|| "facebook_creator".to_string());

    let owner_full_name = v
        .get("uploader")
        .or_else(|| v.get("channel"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());

    let title = v
        .get("title")
        .or_else(|| v.get("fulltitle"))
        .and_then(|x| x.as_str())
        .unwrap_or("Facebook Video")
        .to_string();

    let caption = v
        .get("description")
        .or_else(|| v.get("title"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let timestamp = v.get("timestamp").and_then(|x| x.as_i64());
    let duration_seconds = v.get("duration").and_then(|x| x.as_f64());
    let view_count = v.get("view_count").and_then(|x| x.as_u64());
    let like_count = v.get("like_count").and_then(|x| x.as_u64());

    let thumbnail_url = v
        .get("thumbnail")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            v.get("thumbnails")
                .and_then(|t| t.as_array())
                .and_then(|arr| arr.last())
                .and_then(|item| item.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        });

    let mut slides = Vec::new();
    let is_playlist_root = v.get("_type").and_then(|x| x.as_str()) == Some("playlist");

    if let Some(entries) = v.get("entries").and_then(|e| e.as_array()) {
        for (i, entry) in entries.iter().enumerate() {
            if entry.is_null() {
                continue;
            }
            let ext = entry
                .get("ext")
                .and_then(|x| x.as_str())
                .unwrap_or("mp4")
                .to_string();
            let is_video = ext == "mp4"
                || ext == "webm"
                || entry.get("vcodec").and_then(|x| x.as_str()).unwrap_or("none") != "none";
            let width = entry.get("width").and_then(|x| x.as_u64()).unwrap_or(1080) as u32;
            let height = entry.get("height").and_then(|x| x.as_u64()).unwrap_or(1920) as u32;
            let display_url = entry
                .get("thumbnail")
                .or_else(|| entry.get("url"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let video_url = if is_video {
                entry.get("url").and_then(|x| x.as_str()).map(|s| s.to_string())
            } else {
                None
            };
            let entry_dur = entry.get("duration").and_then(|x| x.as_f64());

            slides.push(CarouselSlideItem {
                index: i + 1,
                media_type: if is_video {
                    "video".to_string()
                } else {
                    "image".to_string()
                },
                width,
                height,
                display_url,
                video_url,
                duration_seconds: entry_dur,
                ext,
            });
        }
    }

    let is_reel_hint = norm.url_type.kind_slug() == "reel";

    if slides.is_empty() {
        if is_playlist_root {
            return Err(AppError::EngineParseError(
                "Facebook post playlist had no video entries; falling back to gallery-dl"
                    .to_string(),
            ));
        }

        let default_w = if is_reel_hint { 1080 } else { 1920 };
        let default_h = if is_reel_hint { 1920 } else { 1080 };
        let width = v.get("width").and_then(|x| x.as_u64()).unwrap_or(default_w) as u32;
        let height = v.get("height").and_then(|x| x.as_u64()).unwrap_or(default_h) as u32;
        let display_url = thumbnail_url.clone().unwrap_or_default();
        let video_url = v.get("url").and_then(|x| x.as_str()).map(|s| s.to_string());

        if display_url.is_empty() && video_url.is_none() && v.get("formats").is_none() {
            return Err(AppError::MediaDeletedOrUnavailable);
        }

        slides.push(CarouselSlideItem {
            index: 1,
            media_type: "video".to_string(),
            width,
            height,
            display_url,
            video_url,
            duration_seconds,
            ext: "mp4".to_string(),
        });
    }

    let first_slide = slides.first();
    let aspect_ratio_label = first_slide
        .map(|s| compute_aspect_ratio_label(s.width, s.height, is_reel_hint))
        .unwrap_or_else(|| {
            if is_reel_hint {
                "9:16".to_string()
            } else {
                "16:9".to_string()
            }
        });

    let format_options = extract_facebook_format_matrix(&v, duration_seconds);
    let available_qualities: Vec<String> = format_options
        .iter()
        .map(|o| o.quality_key.clone())
        .collect();

    let is_carousel = slides.len() > 1;
    let url_type = if is_carousel {
        "carousel".to_string()
    } else {
        norm.url_type.kind_slug().to_string()
    };
    let type_badge = if is_carousel {
        "MULTI-POST".to_string()
    } else {
        norm.type_badge.clone()
    };

    Ok(MediaInspectionResult {
        canonical_url: v
            .get("webpage_url")
            .and_then(|x| x.as_str())
            .filter(|u| u.contains("facebook.com"))
            .map(|s| s.to_string())
            .unwrap_or_else(|| norm.canonical_url.clone()),
        shortcode: norm
            .shortcode
            .clone()
            .or_else(|| v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())),
        url_type,
        type_badge,
        owner_username,
        owner_full_name,
        title,
        caption,
        timestamp,
        thumbnail_url: slides
            .first()
            .and_then(|s| {
                if s.display_url.is_empty() {
                    None
                } else {
                    Some(s.display_url.clone())
                }
            })
            .or(thumbnail_url),
        duration_seconds,
        view_count,
        like_count,
        aspect_ratio_label,
        total_items: slides.len(),
        slides,
        available_qualities,
        format_options,
    })
}

/// Parses `gallery-dl --dump-json` output for Facebook photo albums or multi-media posts into `MediaInspectionResult`.
pub fn parse_gallery_dl_inspect_json(
    raw_json: &str,
    norm: &NormalizedFacebookUrl,
) -> AppResult<MediaInspectionResult> {
    let root: Value = serde_json::from_str(raw_json)
        .map_err(|e| AppError::EngineParseError(format!("Invalid gallery-dl JSON: {}", e)))?;

    let arr = root.as_array().ok_or_else(|| {
        AppError::EngineParseError("Expected JSON array from gallery-dl".to_string())
    })?;

    let mut owner_username = norm
        .username_hint
        .clone()
        .unwrap_or_else(|| "facebook_creator".to_string());
    let mut owner_full_name: Option<String> = None;
    let mut caption = String::new();
    let mut timestamp: Option<i64> = None;
    let mut shortcode = norm.shortcode.clone();
    let mut slides = Vec::new();

    for item in arr {
        if let Some(tuple) = item.as_array() {
            let msg_type = tuple.first().and_then(|x| x.as_i64()).unwrap_or(0);
            let meta_opt = if msg_type == 2 {
                tuple.get(1)
            } else if msg_type == 3 {
                tuple.get(2)
            } else {
                None
            };

            if let Some(meta) = meta_opt {
                if let Some(u) = meta
                    .get("username")
                    .or_else(|| meta.get("author"))
                    .or_else(|| meta.get("user"))
                    .and_then(|x| x.as_str())
                {
                    if !u.is_empty() {
                        owner_username = u.to_string();
                    }
                }
                if owner_full_name.is_none() {
                    owner_full_name = meta
                        .get("author")
                        .or_else(|| meta.get("fullname"))
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                }
                if caption.is_empty() {
                    if let Some(desc) = meta
                        .get("description")
                        .or_else(|| meta.get("content"))
                        .and_then(|x| x.as_str())
                    {
                        caption = desc.to_string();
                    }
                }
                if shortcode.is_none() {
                    shortcode = meta
                        .get("post_id")
                        .or_else(|| meta.get("id"))
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                }
                if timestamp.is_none() {
                    timestamp = meta.get("date").and_then(|x| x.as_i64());
                }

                if msg_type == 3 {
                    let direct_url = tuple
                        .get(1)
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string();
                    let video_url = meta
                        .get("video_url")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                    let ext = meta
                        .get("extension")
                        .and_then(|x| x.as_str())
                        .unwrap_or(if video_url.is_some() { "mp4" } else { "jpg" })
                        .to_string();
                    let is_video = video_url.is_some() || ext == "mp4";
                    let display_url = meta
                        .get("display_url")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| direct_url.clone());
                    let width = meta.get("width").and_then(|x| x.as_u64()).unwrap_or(1080) as u32;
                    let height = meta.get("height").and_then(|x| x.as_u64()).unwrap_or(1080) as u32;
                    let idx = slides.len() + 1;

                    slides.push(CarouselSlideItem {
                        index: idx,
                        media_type: if is_video {
                            "video".to_string()
                        } else {
                            "image".to_string()
                        },
                        width,
                        height,
                        display_url,
                        video_url: if is_video {
                            video_url.or(Some(direct_url))
                        } else {
                            None
                        },
                        duration_seconds: meta.get("duration").and_then(|x| x.as_f64()),
                        ext,
                    });
                }
            }
        }
    }

    if slides.is_empty() {
        return Err(AppError::MediaDeletedOrUnavailable);
    }

    let is_carousel = slides.len() > 1;
    let url_type = if is_carousel {
        "carousel".to_string()
    } else {
        norm.url_type.kind_slug().to_string()
    };
    let type_badge = if is_carousel {
        "ALBUM".to_string()
    } else {
        norm.type_badge.clone()
    };
    let thumbnail_url = slides.first().map(|s| s.display_url.clone());

    let default_formats = vec![VideoFormatOption {
        quality_key: "best".to_string(),
        format_id: "best".to_string(),
        resolution_label: "Original Resolution".to_string(),
        badge: "ORIGINAL".to_string(),
        vcodec: "Original".to_string(),
        acodec: "Original".to_string(),
        ext: slides.first().map(|s| s.ext.clone()).unwrap_or_else(|| "jpg".to_string()),
        height: slides.first().map(|s| s.height),
        fps: None,
        filesize_bytes: None,
        is_dash: false,
        is_audio_only: false,
    }];

    Ok(MediaInspectionResult {
        canonical_url: norm.canonical_url.clone(),
        shortcode,
        url_type,
        type_badge,
        owner_username: owner_username.clone(),
        owner_full_name,
        title: if caption.is_empty() {
            format!("Facebook Post by {}", owner_username)
        } else {
            caption.chars().take(72).collect()
        },
        caption,
        timestamp,
        thumbnail_url,
        duration_seconds: slides.first().and_then(|s| s.duration_seconds),
        view_count: None,
        like_count: None,
        aspect_ratio_label: "1:1".to_string(),
        total_items: slides.len(),
        slides,
        available_qualities: vec!["best".to_string()],
        format_options: default_formats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::url_normalizer::normalize_facebook_url;

    #[test]
    fn test_parse_ytdlp_progress_and_merger_lines() {
        let mut stage = JobStatus::DownloadingVideo;
        let line1 = "[download]  64.5% of   42.80MiB at    8.50MiB/s ETA 00:02";
        let p1 = parse_ytdlp_stdout_line(line1, "job-fb-1", 1, 1, &mut stage).unwrap();
        assert_eq!(p1.job_id, "job-fb-1");
        assert!((p1.percentage - 64.5).abs() < 0.01);
        assert!(p1.total_bytes.unwrap() > 42_000_000);
        assert!(p1.speed_bps.unwrap() > 8_000_000);
        assert_eq!(p1.eta_seconds, Some(2));

        let line2 = "[Merger] Merging formats into \"fb_reel.mp4\"";
        let p2 = parse_ytdlp_stdout_line(line2, "job-fb-1", 1, 1, &mut stage).unwrap();
        assert_eq!(p2.status, JobStatus::Merging);
        assert_eq!(p2.percentage, 99.0);
    }

    #[test]
    fn test_parse_facebook_ytdlp_dash_formats() {
        let norm = normalize_facebook_url("https://www.facebook.com/reel/112233445566").unwrap();
        let sample_json = r#"{
            "id": "112233445566",
            "title": "Epic Drone Reel Over Mountains",
            "description": "Shot in 4K 60fps #reel #facebook",
            "uploader": "Aerial Visuals",
            "uploader_id": "aerialvisuals",
            "timestamp": 1730000000,
            "duration": 24.5,
            "view_count": 154200,
            "like_count": 9820,
            "width": 1080,
            "height": 1920,
            "thumbnail": "https://scontent.xx.fbcdn.net/v/t15.5256-10/thumb.jpg",
            "formats": [
                {
                    "format_id": "sd",
                    "ext": "mp4",
                    "width": 480,
                    "height": 854,
                    "vcodec": "avc1.42c01e",
                    "acodec": "mp4a.40.2",
                    "filesize": 4500000
                },
                {
                    "format_id": "hd",
                    "ext": "mp4",
                    "width": 720,
                    "height": 1280,
                    "vcodec": "avc1.64001f",
                    "acodec": "mp4a.40.2",
                    "filesize": 9200000
                },
                {
                    "format_id": "dash_h264_1080p",
                    "ext": "mp4",
                    "width": 1080,
                    "height": 1920,
                    "fps": 60,
                    "vcodec": "avc1.64002a",
                    "acodec": "none",
                    "filesize": 18500000
                },
                {
                    "format_id": "dash_audio_128k",
                    "ext": "m4a",
                    "vcodec": "none",
                    "acodec": "mp4a.40.5",
                    "filesize": 420000
                }
            ]
        }"#;

        let res = parse_ytdlp_inspect_json(sample_json, &norm).expect("parse fb json");
        assert_eq!(res.shortcode.as_deref(), Some("112233445566"));
        assert_eq!(res.type_badge, "REEL");
        assert_eq!(res.aspect_ratio_label, "9:16");
        assert_eq!(res.view_count, Some(154200));
        assert_eq!(res.like_count, Some(9820));
        assert!(res.available_qualities.contains(&"best".to_string()));
        assert!(res.available_qualities.contains(&"1080p".to_string()));
        assert!(res.available_qualities.contains(&"720p".to_string()));
        assert!(res.available_qualities.contains(&"480p".to_string()));
        assert!(res.available_qualities.contains(&"audio_mp3".to_string()));
        assert!(res.available_qualities.contains(&"audio_m4a".to_string()));
    }
}
