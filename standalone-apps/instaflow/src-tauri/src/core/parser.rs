use crate::core::url_normalizer::NormalizedInstagramUrl;
use crate::error::{AppError, AppResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    pub index: usize, // 1-based slide index
    pub media_type: String, // "image" | "video"
    pub width: u32,
    pub height: u32,
    pub display_url: String,
    pub video_url: Option<String>,
    pub duration_seconds: Option<f64>,
    pub ext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInspectionResult {
    pub canonical_url: String,
    pub shortcode: Option<String>,
    pub url_type: String, // "reel" | "post" | "carousel" | "stories" | "highlight" | "profile"
    pub type_badge: String,
    pub owner_username: String,
    pub owner_full_name: Option<String>,
    pub caption: String,
    pub timestamp: Option<i64>,
    pub thumbnail_url: Option<String>,
    pub total_items: usize,
    pub slides: Vec<CarouselSlideItem>,
    pub available_qualities: Vec<String>,
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

    pub fn emit_if_ready(&mut self, app: Option<&AppHandle>, payload: &ProgressPayload, force: bool) {
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

/// Parses a single `yt-dlp` stdout line into a `ProgressPayload` if it contains progress or muxing telemetry.
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

    // Detect stream transition (video vs audio track)
    if trimmed.starts_with("[download] Destination:") {
        let lower = trimmed.to_lowercase();
        if lower.ends_with(".m4a") || lower.ends_with(".mp3") || lower.contains(".f140.") || lower.contains("audio") {
            *current_stream_stage = JobStatus::DownloadingAudio;
        } else {
            *current_stream_stage = JobStatus::DownloadingVideo;
        }
    }

    if trimmed.starts_with("[Merger]") || trimmed.starts_with("[ExtractAudio]") || trimmed.contains("Merging formats into") {
        *current_stream_stage = JobStatus::Merging;
        return Some(ProgressPayload {
            job_id: job_id.to_string(),
            item_index,
            total_items,
            status: JobStatus::Merging,
            stage_label: "Merging lossless audio + video via FFmpeg...".to_string(),
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
            format!("Downloading slide {} of {}...", item_index, total_items)
        } else if *current_stream_stage == JobStatus::DownloadingAudio {
            "Downloading high-bitrate audio track...".to_string()
        } else {
            "Downloading video stream...".to_string()
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

/// Checks if a stderr line indicates an Instagram rate-limit, challenge, or login wall that should trip the circuit breaker.
pub fn is_circuit_breaker_trigger(stderr_line: &str) -> Option<String> {
    let lower = stderr_line.to_lowercase();
    if lower.contains("http error 429")
        || lower.contains("too many requests")
        || lower.contains("checkpoint_required")
        || lower.contains("challenge_required")
        || lower.contains("please wait a few minutes before you try again")
    {
        return Some(stderr_line.trim().to_string());
    }
    None
}

/// Parses `yt-dlp --dump-single-json` output for a Reel, Video, or Post into `MediaInspectionResult`.
pub fn parse_ytdlp_inspect_json(
    raw_json: &str,
    norm: &NormalizedInstagramUrl,
) -> AppResult<MediaInspectionResult> {
    let v: Value = serde_json::from_str(raw_json)
        .map_err(|e| AppError::EngineParseError(format!("Invalid yt-dlp JSON: {}", e)))?;

    let owner_username = v
        .get("channel")
        .or_else(|| v.get("uploader_id"))
        .or_else(|| v.get("uploader"))
        .and_then(|x| x.as_str())
        .map(|s| s.trim_start_matches('@').to_string())
        .or_else(|| norm.username_hint.clone())
        .unwrap_or_else(|| "instagram_user".to_string());

    let owner_full_name = v
        .get("uploader")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

    let caption = v
        .get("description")
        .or_else(|| v.get("title"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let timestamp = v.get("timestamp").and_then(|x| x.as_i64());
    let thumbnail_url = v
        .get("thumbnail")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());

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
            let height = entry.get("height").and_then(|x| x.as_u64()).unwrap_or(1350) as u32;
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
            let duration_seconds = entry.get("duration").and_then(|x| x.as_f64());

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
                duration_seconds,
                ext,
            });
        }
    }

    if slides.is_empty() {
        if is_playlist_root {
            return Err(AppError::EngineParseError(
                "yt-dlp playlist had no video entries (photo carousel requires GraphQL extractor)"
                    .to_string(),
            ));
        }
        let width = v.get("width").and_then(|x| x.as_u64()).unwrap_or(1080) as u32;
        let height = v.get("height").and_then(|x| x.as_u64()).unwrap_or(1920) as u32;
        let duration_seconds = v.get("duration").and_then(|x| x.as_f64());
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

    let is_carousel = slides.len() > 1;
    let url_type = if is_carousel {
        "carousel".to_string()
    } else {
        norm.url_type.kind_slug().to_string()
    };
    let type_badge = if is_carousel {
        "CAROUSEL".to_string()
    } else {
        norm.type_badge.clone()
    };

    Ok(MediaInspectionResult {
        canonical_url: norm.canonical_url.clone(),
        shortcode: norm
            .shortcode
            .clone()
            .or_else(|| v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string())),
        url_type,
        type_badge,
        owner_username,
        owner_full_name,
        caption,
        timestamp,
        thumbnail_url: slides.first().map(|s| s.display_url.clone()).or(thumbnail_url),
        total_items: slides.len(),
        slides,
        available_qualities: vec![
            "best".to_string(),
            "1080p".to_string(),
            "720p".to_string(),
            "audio_only".to_string(),
        ],
    })
}

/// Parses `gallery-dl --dump-json` output (array of tuple elements `[code, url?, metadata]`) into `MediaInspectionResult`.
pub fn parse_gallery_dl_inspect_json(
    raw_json: &str,
    norm: &NormalizedInstagramUrl,
) -> AppResult<MediaInspectionResult> {
    let root: Value = serde_json::from_str(raw_json)
        .map_err(|e| AppError::EngineParseError(format!("Invalid gallery-dl JSON: {}", e)))?;

    let arr = root
        .as_array()
        .ok_or_else(|| AppError::EngineParseError("Expected JSON array from gallery-dl".to_string()))?;

    let mut owner_username = norm
        .username_hint
        .clone()
        .unwrap_or_else(|| "instagram_user".to_string());
    let mut owner_full_name: Option<String> = None;
    let mut caption = String::new();
    let mut timestamp: Option<i64> = None;
    let mut shortcode = norm.shortcode.clone();
    let mut slides = Vec::new();

    for item in arr {
        if let Some(tuple) = item.as_array() {
            let msg_type = tuple.first().and_then(|x| x.as_i64()).unwrap_or(0);
            // In gallery-dl --dump-json:
            // [2, metadata_obj] is directory/post metadata
            // [3, "https://...", metadata_obj] is an individual downloadable media item (slide)
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
                    .or_else(|| meta.get("owner_username"))
                    .and_then(|x| x.as_str())
                {
                    if !u.is_empty() {
                        owner_username = u.to_string();
                    }
                }
                if owner_full_name.is_none() {
                    owner_full_name = meta
                        .get("fullname")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                }
                if caption.is_empty() {
                    if let Some(desc) = meta.get("description").and_then(|x| x.as_str()) {
                        caption = desc.to_string();
                    }
                }
                if shortcode.is_none() {
                    shortcode = meta
                        .get("post_shortcode")
                        .or_else(|| meta.get("shortcode"))
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
                    let height = meta.get("height").and_then(|x| x.as_u64()).unwrap_or(1350) as u32;
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
        "CAROUSEL".to_string()
    } else {
        norm.type_badge.clone()
    };
    let thumbnail_url = slides.first().map(|s| s.display_url.clone());

    Ok(MediaInspectionResult {
        canonical_url: norm.canonical_url.clone(),
        shortcode,
        url_type,
        type_badge,
        owner_username,
        owner_full_name,
        caption,
        timestamp,
        thumbnail_url,
        total_items: slides.len(),
        slides,
        available_qualities: vec![
            "best".to_string(),
            "1080p".to_string(),
            "720p".to_string(),
            "audio_only".to_string(),
        ],
    })
}

fn extract_best_graphql_image_url(node: &Value) -> String {
    if let Some(candidates) = node
        .pointer("/image_versions2/candidates")
        .and_then(|c| c.as_array())
    {
        // First try to pick the candidate with the largest explicit width * height
        let mut best_sized: Option<(u64, &str)> = None;
        for cand in candidates {
            if let (Some(w), Some(h), Some(u)) = (
                cand.get("width").and_then(|x| x.as_u64()),
                cand.get("height").and_then(|x| x.as_u64()),
                cand.get("url").and_then(|x| x.as_str()),
            ) {
                let area = w.saturating_mul(h);
                if area > 0 && best_sized.map(|(b, _)| area > b).unwrap_or(true) {
                    best_sized = Some((area, u));
                }
            }
        }
        if let Some((_, url)) = best_sized {
            return url.to_string();
        }

        // Otherwise pick the first candidate without downscaled _s{W}x{H} / _p{W}x{H} in stp=, or candidates[0]
        for cand in candidates {
            if let Some(u) = cand.get("url").and_then(|x| x.as_str()) {
                if !u.contains("_s150x150")
                    && !u.contains("_s240x240")
                    && !u.contains("_s320x320")
                    && !u.contains("_s480x480")
                    && !u.contains("_s640x640")
                    && !u.contains("_s720x720")
                    && !u.contains("_s750x750")
                {
                    return u.to_string();
                }
            }
        }
        if let Some(first_url) = candidates
            .first()
            .and_then(|c| c.get("url"))
            .and_then(|x| x.as_str())
        {
            return first_url.to_string();
        }
    }

    node.get("display_uri")
        .or_else(|| node.get("display_url"))
        .or_else(|| node.get("thumbnail_src"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn extract_graphql_slide(node: &Value, index: usize) -> Option<CarouselSlideItem> {
    let media_type_num = node.get("media_type").and_then(|x| x.as_u64()).unwrap_or(1);
    let typename = node
        .get("__typename")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    let is_video_flag = node.get("is_video").and_then(|x| x.as_bool()).unwrap_or(false);

    let video_url = node
        .get("video_versions")
        .and_then(|vv| vv.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("url"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            node.get("video_url")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        });

    let is_video = media_type_num == 2
        || typename.contains("Video")
        || is_video_flag
        || video_url.is_some();

    let display_url = extract_best_graphql_image_url(node);
    if display_url.is_empty() && video_url.is_none() {
        return None;
    }

    let width = node
        .get("original_width")
        .or_else(|| node.pointer("/dimensions/width"))
        .and_then(|x| x.as_u64())
        .unwrap_or(1080) as u32;
    let height = node
        .get("original_height")
        .or_else(|| node.pointer("/dimensions/height"))
        .and_then(|x| x.as_u64())
        .unwrap_or(if is_video { 1920 } else { 1080 }) as u32;

    let duration_seconds = node.get("video_duration").and_then(|x| x.as_f64());

    Some(CarouselSlideItem {
        index,
        media_type: if is_video {
            "video".to_string()
        } else {
            "image".to_string()
        },
        width,
        height,
        display_url,
        video_url,
        duration_seconds,
        ext: if is_video {
            "mp4".to_string()
        } else {
            "jpg".to_string()
        },
    })
}

/// Parses Instagram's Polaris/GraphQL response (`xig_polaris_media` / `xdt_shortcode_media` / `shortcode_media`)
/// dumped by `yt-dlp --write-pages --skip-download` into a complete `MediaInspectionResult` (supporting single photos,
/// multi-item photo/video carousels, and Reels without requiring login cookies).
pub fn parse_instagram_graphql_dump_json(
    raw_json: &str,
    norm: &NormalizedInstagramUrl,
) -> AppResult<MediaInspectionResult> {
    let v: Value = serde_json::from_str(raw_json)
        .map_err(|e| AppError::EngineParseError(format!("Invalid Instagram GraphQL JSON: {}", e)))?;

    let media_root = v
        .pointer("/data/xig_polaris_media/if_not_gated_logged_out")
        .filter(|x| !x.is_null() && x.is_object())
        .or_else(|| {
            v.pointer("/data/xig_polaris_media")
                .filter(|x| !x.is_null() && x.is_object())
        })
        .or_else(|| {
            v.pointer("/data/xdt_shortcode_media")
                .filter(|x| !x.is_null() && x.is_object())
        })
        .or_else(|| {
            v.pointer("/data/shortcode_media")
                .filter(|x| !x.is_null() && x.is_object())
        })
        .ok_or(AppError::MediaDeletedOrUnavailable)?;

    let owner_username = media_root
        .pointer("/user/username")
        .or_else(|| media_root.pointer("/owner/username"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| norm.username_hint.clone())
        .unwrap_or_else(|| "instagram_user".to_string());

    let owner_full_name = media_root
        .pointer("/user/full_name")
        .or_else(|| media_root.pointer("/owner/full_name"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let caption = media_root
        .pointer("/caption/text")
        .or_else(|| {
            media_root
                .pointer("/edge_media_to_caption/edges/0/node/text")
        })
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    let timestamp = media_root
        .get("taken_at")
        .or_else(|| media_root.get("taken_at_timestamp"))
        .and_then(|x| x.as_i64());

    let shortcode = media_root
        .get("code")
        .or_else(|| media_root.get("shortcode"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .or_else(|| norm.shortcode.clone());

    let mut slides = Vec::new();

    if let Some(carousel_arr) = media_root.get("carousel_media").and_then(|x| x.as_array()) {
        for (i, slide_node) in carousel_arr.iter().enumerate() {
            if let Some(slide) = extract_graphql_slide(slide_node, i + 1) {
                slides.push(slide);
            }
        }
    } else if let Some(edges_arr) = media_root
        .pointer("/edge_sidecar_to_children/edges")
        .and_then(|x| x.as_array())
    {
        for (i, edge) in edges_arr.iter().enumerate() {
            if let Some(node) = edge.get("node") {
                if let Some(slide) = extract_graphql_slide(node, i + 1) {
                    slides.push(slide);
                }
            }
        }
    }

    if slides.is_empty() {
        if let Some(single_slide) = extract_graphql_slide(media_root, 1) {
            slides.push(single_slide);
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
        "CAROUSEL".to_string()
    } else {
        norm.type_badge.clone()
    };
    let thumbnail_url = slides.first().map(|s| s.display_url.clone());

    Ok(MediaInspectionResult {
        canonical_url: norm.canonical_url.clone(),
        shortcode,
        url_type,
        type_badge,
        owner_username,
        owner_full_name,
        caption,
        timestamp,
        thumbnail_url,
        total_items: slides.len(),
        slides,
        available_qualities: vec![
            "best".to_string(),
            "1080p".to_string(),
            "720p".to_string(),
            "audio_only".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::url_normalizer::normalize_instagram_url;

    #[test]
    fn test_parse_ytdlp_progress_and_merger_lines() {
        let mut stage = JobStatus::DownloadingVideo;
        let line1 = "[download]  45.2% of   18.40MiB at    4.12MiB/s ETA 00:02";
        let p1 = parse_ytdlp_stdout_line(line1, "job-1", 1, 1, &mut stage).unwrap();
        assert_eq!(p1.job_id, "job-1");
        assert!((p1.percentage - 45.2).abs() < 0.01);
        assert!(p1.total_bytes.unwrap() > 18_000_000);
        assert!(p1.speed_bps.unwrap() > 4_000_000);
        assert_eq!(p1.eta_seconds, Some(2));

        let line2 = "[Merger] Merging formats into \"output.mp4\"";
        let p2 = parse_ytdlp_stdout_line(line2, "job-1", 1, 1, &mut stage).unwrap();
        assert_eq!(p2.status, JobStatus::Merging);
        assert_eq!(p2.percentage, 99.0);
    }

    #[test]
    fn test_parse_gallery_dl_mixed_carousel_json() {
        let sample = r#"[
            [2, {"username": "visual.arts", "fullname": "Visual Arts Studio", "description": "Mixed carousel #art", "post_shortcode": "C9Carousel1"}],
            [3, "https://scontent.cdninstagram.com/v/photo1.jpg", {"username": "visual.arts", "width": 1080, "height": 1350, "extension": "jpg", "display_url": "https://scontent.cdninstagram.com/v/photo1.jpg"}],
            [3, "https://scontent.cdninstagram.com/v/video2.mp4", {"username": "visual.arts", "width": 1080, "height": 1350, "extension": "mp4", "video_url": "https://scontent.cdninstagram.com/v/video2.mp4", "display_url": "https://scontent.cdninstagram.com/v/thumb2.jpg"}]
        ]"#;

        let norm = normalize_instagram_url("https://www.instagram.com/p/C9Carousel1/").unwrap();
        let res = parse_gallery_dl_inspect_json(sample, &norm).unwrap();
        assert_eq!(res.owner_username, "visual.arts");
        assert_eq!(res.type_badge, "CAROUSEL");
        assert_eq!(res.total_items, 2);
        assert_eq!(res.slides[0].media_type, "image");
        assert_eq!(res.slides[1].media_type, "video");
    }

    #[test]
    fn test_parse_instagram_graphql_dump_json() {
        let sample = r#"{
            "data": {
                "xig_polaris_media": {
                    "__typename": "XIGPolarisCarouselMedia",
                    "code": "DfXZgm6kd0g",
                    "if_not_gated_logged_out": {
                        "__typename": "XIGPolarisCarouselMedia",
                        "code": "DfXZgm6kd0g",
                        "taken_at": 1793894001,
                        "media_type": 8,
                        "user": {
                            "username": "gurbanisikhi",
                            "full_name": "Gurbani Sikhi"
                        },
                        "caption": {
                            "text": "November 8, 2026 marks Bandi Chhor Diwas"
                        },
                        "carousel_media": [
                            {
                                "__typename": "XIGPolarisImageMedia",
                                "code": "DfXZf2Mx5PO",
                                "media_type": 1,
                                "original_width": 1080,
                                "original_height": 1080,
                                "image_versions2": {
                                    "candidates": [
                                        {"url": "https://instagram.fixc5-1.fna.fbcdn.net/v/slide1_1080.jpg?stp=dst-jpg_e35_tt6"},
                                        {"url": "https://instagram.fixc5-1.fna.fbcdn.net/v/slide1_640.jpg?stp=dst-jpg_e35_s640x640_tt6"}
                                    ]
                                }
                            },
                            {
                                "__typename": "XIGPolarisImageMedia",
                                "code": "DfXZf38xvuB",
                                "media_type": 1,
                                "original_width": 1080,
                                "original_height": 1080,
                                "image_versions2": {
                                    "candidates": [
                                        {"url": "https://instagram.fixc5-2.fna.fbcdn.net/v/slide2_1080.jpg?stp=dst-jpg_e35_tt6"}
                                    ]
                                }
                            }
                        ]
                    }
                }
            }
        }"#;

        let norm = normalize_instagram_url(
            "https://www.instagram.com/p/DfXZgm6kd0g/?utm_source=ig_web_copy_link&cplk=NTc4MTIwNjQ2YQ==",
        )
        .unwrap();
        let res = parse_instagram_graphql_dump_json(sample, &norm).unwrap();
        assert_eq!(res.owner_username, "gurbanisikhi");
        assert_eq!(res.owner_full_name.as_deref(), Some("Gurbani Sikhi"));
        assert_eq!(res.type_badge, "CAROUSEL");
        assert_eq!(res.total_items, 2);
        assert_eq!(res.slides[0].media_type, "image");
        assert_eq!(res.slides[0].width, 1080);
        assert!(res.slides[0].display_url.contains("slide1_1080.jpg"));
    }
}
