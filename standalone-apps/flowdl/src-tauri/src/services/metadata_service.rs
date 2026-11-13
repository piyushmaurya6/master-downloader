use crate::models::{MediaMetadata, PlaylistEntryPreview, SubtitleTrack, VideoFormatOption};
use crate::services::error_handler::classify_error;
use crate::utils::{find_bundled_deno_path, no_window_command, DiagnosticLogger};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

pub fn validate_media_url(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("Invalid URL: URL cannot be empty.".to_string());
    }
    if trimmed.starts_with('-') {
        return Err("Invalid URL: URL cannot start with a hyphen.".to_string());
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("Invalid URL: Only http:// and https:// links are supported.".to_string());
    }
    if trimmed.len() > 2048 {
        return Err("Invalid URL: Link exceeds maximum supported length.".to_string());
    }
    Ok(trimmed.to_string())
}

pub async fn analyze_url_metadata(
    ytdlp_path: &Path,
    raw_url: &str,
    logger: &DiagnosticLogger,
) -> Result<MediaMetadata, String> {
    let clean_url = validate_media_url(raw_url)?;
    logger.info("MetadataService", &format!("Analyzing URL: {}", clean_url));

    let mut args: Vec<String> = vec![
        "--dump-single-json".to_string(),
        "--no-warnings".to_string(),
        "--flat-playlist".to_string(),
        "--socket-timeout".to_string(),
        "20".to_string(),
    ];

    if let Some(deno_path) = find_bundled_deno_path() {
        args.push("--js-runtimes".to_string());
        args.push(format!("deno:{}", deno_path.to_string_lossy()));
    }

    args.push("--".to_string());
    args.push(clean_url.clone());

    let output = no_window_command(ytdlp_path)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to launch yt-dlp for metadata analysis: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let combined = if stderr.trim().is_empty() { stdout } else { stderr };
        logger.error("MetadataService", &format!("Analysis failed: {}", combined));
        let classified = classify_error(&combined);
        return Err(format!("{}||{}", classified.category, classified.user_message));
    }

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    parse_ytdlp_json(&stdout_str, &clean_url)
}

pub fn parse_ytdlp_json(json_str: &str, fallback_url: &str) -> Result<MediaMetadata, String> {
    let root: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse metadata JSON: {}", e))?;

    let is_playlist = root.get("_type").and_then(|v| v.as_str()) == Some("playlist")
        || root.get("entries").and_then(|v| v.as_array()).is_some();

    let id = root
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let title = root
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled Media")
        .to_string();

    let webpage_url = root
        .get("webpage_url")
        .or_else(|| root.get("original_url"))
        .and_then(|v| v.as_str())
        .unwrap_or(fallback_url)
        .to_string();

    let thumbnail = root
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            root.get("thumbnails")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.last())
                .and_then(|t| t.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        });

    let uploader = root
        .get("uploader")
        .or_else(|| root.get("channel"))
        .or_else(|| root.get("Creator"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let channel_url = root
        .get("channel_url")
        .or_else(|| root.get("uploader_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let duration = root.get("duration").and_then(|v| v.as_f64());

    let duration_string = root
        .get("duration_string")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| match duration {
            Some(d) if d > 0.0 => {
                let total = d.round() as u64;
                let h = total / 3600;
                let m = (total % 3600) / 60;
                let s = total % 60;
                if h > 0 {
                    format!("{}:{:02}:{:02}", h, m, s)
                } else {
                    format!("{}:{:02}", m, s)
                }
            }
            _ => "—".to_string(),
        });

    let extractor = root
        .get("extractor_key")
        .or_else(|| root.get("extractor"))
        .and_then(|v| v.as_str())
        .unwrap_or("Web")
        .to_string();

    let upload_date = root
        .get("upload_date")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let description = root
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.chars().count() > 500 {
                let truncated: String = s.chars().take(500).collect();
                format!("{}…", truncated)
            } else {
                s.to_string()
            }
        });

    let mut playlist_entries = Vec::new();
    let mut playlist_count = None;
    let mut playlist_title = None;

    if is_playlist {
        playlist_title = Some(title.clone());
        if let Some(entries) = root.get("entries").and_then(|v| v.as_array()) {
            playlist_count = Some(
                root.get("playlist_count")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(entries.len()),
            );
            for entry in entries.iter().take(50) {
                let entry_id = entry
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let entry_title = entry
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Untitled video")
                    .to_string();
                let entry_url = entry
                    .get("url")
                    .or_else(|| entry.get("webpage_url"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        if !entry_id.is_empty() {
                            format!("https://www.youtube.com/watch?v={}", entry_id)
                        } else {
                            webpage_url.clone()
                        }
                    });
                let entry_thumb = entry
                    .get("thumbnail")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        entry
                            .get("thumbnails")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.last())
                            .and_then(|t| t.get("url"))
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string())
                    });

                playlist_entries.push(PlaylistEntryPreview {
                    id: entry_id,
                    title: entry_title,
                    duration: entry.get("duration").and_then(|v| v.as_f64()),
                    url: entry_url,
                    thumbnail: entry_thumb,
                    uploader: entry
                        .get("uploader")
                        .or_else(|| entry.get("channel"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
    }

    let mut heights_set = BTreeSet::new();
    let mut codecs_set = BTreeSet::new();
    let mut fps_set = BTreeSet::new();
    let mut formats_list = Vec::new();
    let mut audio_formats_list = Vec::new();
    let mut estimated_sizes_by_height: HashMap<u32, u64> = HashMap::new();
    let mut best_audio_size: u64 = 0;

    if let Some(raw_formats) = root.get("formats").and_then(|v| v.as_array()) {
        for f in raw_formats {
            let format_id = f
                .get("format_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let ext = f
                .get("ext")
                .and_then(|v| v.as_str())
                .unwrap_or("mp4")
                .to_string();
            let vcodec = f
                .get("vcodec")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let acodec = f
                .get("acodec")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            let has_video = vcodec.as_deref().map(|c| c != "none").unwrap_or(false);
            let has_audio = acodec.as_deref().map(|c| c != "none").unwrap_or(false);

            let height = f.get("height").and_then(|v| v.as_u64()).map(|n| n as u32);
            let width = f.get("width").and_then(|v| v.as_u64()).map(|n| n as u32);
            let fps = f
                .get("fps")
                .and_then(|v| v.as_f64())
                .map(|n| n.round() as u32)
                .filter(|&n| n > 0);

            let filesize = f
                .get("filesize")
                .or_else(|| f.get("filesize_approx"))
                .and_then(|v| v.as_u64());
            let tbr = f.get("tbr").and_then(|v| v.as_f64());

            let resolution = f
                .get("resolution")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| match (width, height) {
                    (Some(w), Some(h)) => format!("{}x{}", w, h),
                    (_, Some(h)) => format!("{}p", h),
                    _ => if has_audio && !has_video { "audio only".to_string() } else { "unknown".to_string() },
                });

            if has_video {
                if let Some(h) = height {
                    if h >= 144 {
                        heights_set.insert(h);
                        if let Some(sz) = filesize {
                            let current = estimated_sizes_by_height.entry(h).or_insert(0);
                            if sz > *current {
                                *current = sz;
                            }
                        }
                    }
                }
                if let Some(ref vc) = vcodec {
                    let vc_lower = vc.to_lowercase();
                    if vc_lower.starts_with("avc1") || vc_lower.contains("h264") {
                        codecs_set.insert("h264".to_string());
                    } else if vc_lower.starts_with("hev1")
                        || vc_lower.starts_with("hvc1")
                        || vc_lower.contains("h265")
                        || vc_lower.contains("hevc")
                    {
                        codecs_set.insert("h265".to_string());
                    } else if vc_lower.starts_with("vp9") || vc_lower.starts_with("vp09") {
                        codecs_set.insert("vp9".to_string());
                    } else if vc_lower.starts_with("av01") || vc_lower.contains("av1") {
                        codecs_set.insert("av1".to_string());
                    }
                }
                if let Some(fps_val) = fps {
                    fps_set.insert(fps_val);
                }
            } else if has_audio {
                if let Some(sz) = filesize {
                    if sz > best_audio_size {
                        best_audio_size = sz;
                    }
                }
            }

            let option = VideoFormatOption {
                format_id,
                ext,
                resolution,
                height,
                width,
                fps,
                vcodec,
                acodec,
                filesize_approx: filesize,
                tbr,
                has_video,
                has_audio,
                note: f
                    .get("format_note")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };

            if has_audio && !has_video {
                audio_formats_list.push(option.clone());
            }
            if has_video || has_audio {
                formats_list.push(option);
            }
        }
    }

    // Add best audio stream size to video-only estimated sizes
    for (_h, sz) in estimated_sizes_by_height.iter_mut() {
        *sz = sz.saturating_add(best_audio_size);
    }

    // If playlist (flat) had no individual format list, provide standard heights so user can pick max resolution
    let mut available_heights: Vec<u32> = heights_set.into_iter().rev().collect();
    if available_heights.is_empty() && is_playlist {
        available_heights = vec![2160, 1440, 1080, 720, 480, 360];
    }

    let mut subtitles = Vec::new();
    if let Some(subs_obj) = root.get("subtitles").and_then(|v| v.as_object()) {
        for (lang, tracks) in subs_obj {
            let name = tracks
                .as_array()
                .and_then(|a| a.first())
                .and_then(|t| t.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(lang)
                .to_string();
            subtitles.push(SubtitleTrack {
                lang: lang.clone(),
                name,
                is_auto_generated: false,
            });
        }
    }

    let final_thumb = thumbnail.or_else(|| {
        playlist_entries
            .first()
            .and_then(|e| e.thumbnail.clone())
    });

    Ok(MediaMetadata {
        id,
        title,
        url: fallback_url.to_string(),
        webpage_url,
        thumbnail: final_thumb,
        uploader,
        channel_url,
        duration,
        duration_string,
        extractor,
        upload_date,
        description,
        is_playlist,
        playlist_title,
        playlist_count,
        playlist_entries,
        available_heights,
        available_video_codecs: codecs_set.into_iter().collect(),
        available_fps: fps_set.into_iter().rev().collect(),
        available_audio_formats: audio_formats_list,
        subtitles,
        formats: formats_list,
        estimated_sizes_by_height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_media_url() {
        assert!(validate_media_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").is_ok());
        assert!(validate_media_url("   ").is_err());
        assert!(validate_media_url("--exec calc.exe").is_err());
        assert!(validate_media_url("ftp://files.example.com/video.mp4").is_err());
    }

    #[test]
    fn test_parse_ytdlp_json_extracts_real_heights_and_codecs() {
        let sample_json = r#"{
            "id": "abc123xyz",
            "title": "Sample 1080p Test Video",
            "webpage_url": "https://www.youtube.com/watch?v=abc123xyz",
            "uploader": "Test Creator",
            "duration": 125.0,
            "extractor_key": "Youtube",
            "upload_date": "20261001",
            "formats": [
                {
                    "format_id": "140",
                    "ext": "m4a",
                    "vcodec": "none",
                    "acodec": "mp4a.40.2",
                    "filesize": 2000000
                },
                {
                    "format_id": "137",
                    "ext": "mp4",
                    "height": 1080,
                    "width": 1920,
                    "fps": 60,
                    "vcodec": "avc1.640028",
                    "acodec": "none",
                    "filesize": 25000000
                },
                {
                    "format_id": "136",
                    "ext": "mp4",
                    "height": 720,
                    "width": 1280,
                    "fps": 30,
                    "vcodec": "vp9",
                    "acodec": "none",
                    "filesize": 12000000
                }
            ]
        }"#;
        let meta = parse_ytdlp_json(sample_json, "https://www.youtube.com/watch?v=abc123xyz").unwrap();
        assert_eq!(meta.title, "Sample 1080p Test Video");
        assert_eq!(meta.available_heights, vec![1080, 720]);
        assert!(!meta.available_heights.contains(&2160));
        assert!(meta.available_video_codecs.contains(&"h264".to_string()));
        assert!(meta.available_video_codecs.contains(&"vp9".to_string()));
        assert_eq!(meta.estimated_sizes_by_height.get(&1080), Some(&27000000));
    }
}
