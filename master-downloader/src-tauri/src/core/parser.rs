use crate::core::url_normalizer::Platform;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlideMediaType {
    Image,
    Video,
    Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSlide {
    pub index: usize,
    pub media_type: SlideMediaType,
    pub thumbnail_url: Option<String>,
    pub direct_url: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub filesize_approx: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatItem {
    pub format_id: String,
    pub ext: String,
    pub resolution: String,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub vcodec: Option<String>,
    pub acodec: Option<String>,
    pub filesize_bytes: Option<u64>,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    pub lang: String,
    pub name: String,
    pub is_auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterItem {
    pub title: String,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaylistEntryPreview {
    pub index: usize,
    pub id: String,
    pub title: String,
    pub duration_seconds: Option<f64>,
    pub uploader: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInspectionResult {
    pub source_url: String,
    pub platform: Platform,
    pub shortcode: Option<String>,
    pub url_type: String,
    pub author_username: String,
    pub author_full_name: Option<String>,
    pub caption: Option<String>,
    pub timestamp: Option<i64>,
    pub like_count: Option<u64>,
    pub view_count: Option<u64>,
    pub comment_count: Option<u64>,
    pub slide_count: usize,
    pub slides: Vec<MediaSlide>,
    pub available_qualities: Vec<String>,
    pub formats: Vec<FormatItem>,
    pub subtitles: Vec<SubtitleTrack>,
    pub chapters: Vec<ChapterItem>,
    pub is_playlist: bool,
    pub playlist_title: Option<String>,
    pub playlist_count: Option<usize>,
    pub playlist_entries: Vec<PlaylistEntryPreview>,
    pub engine_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPayload {
    pub job_id: String,
    pub slide_index: usize,
    pub status: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub speed_bps: Option<f64>,
    pub eta_seconds: Option<u64>,
    pub percentage: f64,
}

/// Parses `yt-dlp --dump-single-json` output across YouTube, Instagram, Facebook, and generic URLs.
pub fn parse_ytdlp_inspect_json(
    raw_json: &str,
    fallback_shortcode: Option<&str>,
    platform_override: Option<Platform>,
) -> AppResult<MediaInspectionResult> {
    let v: Value = serde_json::from_str(raw_json).map_err(|e| {
        AppError::Internal(format!("Failed to parse yt-dlp JSON metadata: {}", e))
    })?;

    let source_url = v
        .get("webpage_url")
        .or_else(|| v.get("original_url"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    let extractor = v
        .get("extractor_key")
        .or_else(|| v.get("extractor"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_lowercase();

    let platform = platform_override.unwrap_or_else(|| {
        if extractor.contains("youtube") {
            Platform::YouTube
        } else if extractor.contains("instagram") {
            Platform::Instagram
        } else if extractor.contains("facebook") {
            Platform::Facebook
        } else {
            Platform::Generic
        }
    });

    let shortcode = v
        .get("id")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .or_else(|| fallback_shortcode.map(|s| s.to_string()));

    let author_username = v
        .get("uploader_id")
        .or_else(|| v.get("channel"))
        .or_else(|| v.get("uploader"))
        .and_then(|s| s.as_str())
        .map(|s| s.trim_start_matches('@').to_string())
        .unwrap_or_else(|| "creator".to_string());

    let author_full_name = v
        .get("uploader")
        .or_else(|| v.get("channel"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let caption = v
        .get("title")
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| v.get("description").and_then(|s| s.as_str()))
        .map(|s| s.to_string());

    let timestamp = v.get("timestamp").and_then(|n| n.as_i64());
    let like_count = v.get("like_count").and_then(|n| n.as_u64());
    let view_count = v.get("view_count").and_then(|n| n.as_u64());
    let comment_count = v.get("comment_count").and_then(|n| n.as_u64());

    let is_playlist_type = v.get("_type").and_then(|s| s.as_str()) == Some("playlist");
    let entries_arr = v.get("entries").and_then(|e| e.as_array());

    let mut slides = Vec::new();
    let mut playlist_entries = Vec::new();
    let mut is_playlist = false;
    let mut playlist_title = None;
    let mut playlist_count = None;

    if is_playlist_type && platform == Platform::YouTube {
        is_playlist = true;
        playlist_title = v
            .get("title")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
        if let Some(entries) = entries_arr {
            playlist_count = Some(entries.len());
            for (idx, entry) in entries.iter().take(50).enumerate() {
                let id = entry
                    .get("id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = entry
                    .get("title")
                    .and_then(|s| s.as_str())
                    .unwrap_or("Untitled Video")
                    .to_string();
                let dur = entry.get("duration").and_then(|n| n.as_f64());
                let uploader = entry
                    .get("uploader")
                    .or_else(|| entry.get("channel"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                playlist_entries.push(PlaylistEntryPreview {
                    index: idx + 1,
                    id,
                    title,
                    duration_seconds: dur,
                    uploader,
                });
            }
            if let Some(first) = entries.first() {
                slides.push(extract_slide_from_ytdlp_node(first, 1));
            }
        }
    } else if let Some(entries) = entries_arr {
        for entry in entries.iter() {
            if entry.is_null() {
                continue;
            }
            slides.push(extract_slide_from_ytdlp_node(entry, slides.len() + 1));
        }
    }

    if slides.is_empty() {
        if is_playlist_type && platform != Platform::YouTube {
            return Err(AppError::MediaUnavailable(
                "yt-dlp playlist had no valid video entries; falling back to GraphQL or gallery-dl."
                    .into(),
            ));
        }
        slides.push(extract_slide_from_ytdlp_node(&v, 1));
    }

    let (formats, available_qualities) = extract_formats_and_qualities(&v);
    let subtitles = extract_subtitles(&v);
    let chapters = extract_chapters(&v);

    let url_type = if is_playlist {
        "playlist".to_string()
    } else if slides.len() > 1 {
        "carousel".to_string()
    } else if source_url.contains("/shorts/") {
        "shorts".to_string()
    } else if source_url.contains("/reel") {
        "reel".to_string()
    } else if source_url.contains("/watch") && platform == Platform::Facebook {
        "watch".to_string()
    } else {
        "video".to_string()
    };

    Ok(MediaInspectionResult {
        source_url,
        platform,
        shortcode,
        url_type,
        author_username,
        author_full_name,
        caption,
        timestamp,
        like_count,
        view_count,
        comment_count,
        slide_count: slides.len(),
        slides,
        available_qualities,
        formats,
        subtitles,
        chapters,
        is_playlist,
        playlist_title,
        playlist_count,
        playlist_entries,
        engine_used: "yt-dlp".into(),
    })
}

fn extract_slide_from_ytdlp_node(node: &Value, index: usize) -> MediaSlide {
    let ext = node.get("ext").and_then(|s| s.as_str()).unwrap_or("mp4");
    let vcodec = node.get("vcodec").and_then(|s| s.as_str()).unwrap_or("h264");

    let media_type = if matches!(ext, "jpg" | "jpeg" | "png" | "webp" | "heic") || vcodec == "none"
    {
        SlideMediaType::Image
    } else {
        SlideMediaType::Video
    };

    let thumbnail_url = node
        .get("thumbnail")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            node.get("thumbnails")
                .and_then(|t| t.as_array())
                .and_then(|arr| arr.last())
                .and_then(|item| item.get("url"))
                .and_then(|u| u.as_str())
                .map(|s| s.to_string())
        });

    let direct_url = node
        .get("url")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let width = node.get("width").and_then(|n| n.as_u64()).map(|n| n as u32);
    let height = node
        .get("height")
        .and_then(|n| n.as_u64())
        .map(|n| n as u32);
    let duration_seconds = node.get("duration").and_then(|n| n.as_f64());
    let filesize_approx = node
        .get("filesize")
        .or_else(|| node.get("filesize_approx"))
        .and_then(|n| n.as_u64());

    MediaSlide {
        index,
        media_type,
        thumbnail_url,
        direct_url,
        width,
        height,
        duration_seconds,
        filesize_approx,
    }
}

fn extract_formats_and_qualities(v: &Value) -> (Vec<FormatItem>, Vec<String>) {
    let mut formats = Vec::new();
    let mut has_4k = false;
    let mut has_1440 = false;
    let mut has_1080p60 = false;
    let mut has_1080 = false;
    let mut has_720 = false;
    let mut has_480 = false;
    let mut has_360 = false;

    let check_node = |node: &Value,
                      formats: &mut Vec<FormatItem>,
                      has_4k: &mut bool,
                      has_1440: &mut bool,
                      has_1080p60: &mut bool,
                      has_1080: &mut bool,
                      has_720: &mut bool,
                      has_480: &mut bool,
                      has_360: &mut bool| {
        if let Some(fmts) = node.get("formats").and_then(|f| f.as_array()) {
            for f in fmts {
                let format_id = f
                    .get("format_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                let ext = f
                    .get("ext")
                    .and_then(|s| s.as_str())
                    .unwrap_or("mp4")
                    .to_string();
                let height = f.get("height").and_then(|n| n.as_u64()).map(|n| n as u32);
                let width = f.get("width").and_then(|n| n.as_u64()).map(|n| n as u32);
                let fps = f.get("fps").and_then(|n| n.as_f64());
                let vcodec = f
                    .get("vcodec")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                let acodec = f
                    .get("acodec")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                let filesize_bytes = f
                    .get("filesize")
                    .or_else(|| f.get("filesize_approx"))
                    .and_then(|n| n.as_u64());

                let has_video = vcodec.as_deref().unwrap_or("none") != "none";
                let has_audio = acodec.as_deref().unwrap_or("none") != "none";

                if let Some(h) = height {
                    let max_dim = width.unwrap_or(h).max(h);
                    if max_dim >= 2160 || h >= 2160 {
                        *has_4k = true;
                    }
                    if max_dim >= 1440 || h >= 1440 {
                        *has_1440 = true;
                    }
                    if max_dim >= 1080 || h >= 1080 {
                        *has_1080 = true;
                        if fps.unwrap_or(30.0) >= 50.0 {
                            *has_1080p60 = true;
                        }
                    }
                    if max_dim >= 720 || h >= 720 {
                        *has_720 = true;
                    }
                    if max_dim >= 480 || h >= 480 {
                        *has_480 = true;
                    }
                    if max_dim >= 360 || h >= 360 {
                        *has_360 = true;
                    }
                }

                let resolution = if let (Some(w), Some(h)) = (width, height) {
                    format!("{}x{}", w, h)
                } else if let Some(h) = height {
                    format!("{}p", h)
                } else if has_audio && !has_video {
                    "audio only".to_string()
                } else {
                    "auto".to_string()
                };

                formats.push(FormatItem {
                    format_id,
                    ext,
                    resolution,
                    height,
                    fps,
                    vcodec,
                    acodec,
                    filesize_bytes,
                    has_video,
                    has_audio,
                });
            }
        }
    };

    check_node(
        v,
        &mut formats,
        &mut has_4k,
        &mut has_1440,
        &mut has_1080p60,
        &mut has_1080,
        &mut has_720,
        &mut has_480,
        &mut has_360,
    );

    if let Some(entries) = v.get("entries").and_then(|e| e.as_array()) {
        for e in entries.iter().take(3) {
            check_node(
                e,
                &mut formats,
                &mut has_4k,
                &mut has_1440,
                &mut has_1080p60,
                &mut has_1080,
                &mut has_720,
                &mut has_480,
                &mut has_360,
            );
        }
    }

    let mut qualities = vec!["best".to_string()];
    if has_4k {
        qualities.push("4k".to_string());
    }
    if has_1440 {
        qualities.push("1440p".to_string());
    }
    if has_1080p60 {
        qualities.push("1080p60".to_string());
    }
    if has_1080 || (!has_720 && !has_480) {
        qualities.push("1080p".to_string());
    }
    if has_720 || qualities.len() <= 2 {
        qualities.push("720p".to_string());
    }
    if has_480 {
        qualities.push("480p".to_string());
    }
    if has_360 {
        qualities.push("360p".to_string());
    }
    qualities.push("audio_mp3".to_string());
    qualities.push("audio_m4a".to_string());
    qualities.push("audio_flac".to_string());
    qualities.push("audio_wav".to_string());
    qualities.push("audio_opus".to_string());

    (formats, qualities)
}

fn extract_subtitles(v: &Value) -> Vec<SubtitleTrack> {
    let mut tracks = Vec::new();
    if let Some(subs) = v.get("subtitles").and_then(|s| s.as_object()) {
        for (lang, arr) in subs {
            let name = arr
                .as_array()
                .and_then(|a| a.first())
                .and_then(|item| item.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or(lang)
                .to_string();
            tracks.push(SubtitleTrack {
                lang: lang.clone(),
                name,
                is_auto: false,
            });
        }
    }
    if let Some(auto_subs) = v.get("automatic_captions").and_then(|s| s.as_object()) {
        for lang in ["en", "en-orig", "es", "fr", "de", "hi", "ja", "ko"] {
            if auto_subs.contains_key(lang) && !tracks.iter().any(|t| t.lang == lang) {
                tracks.push(SubtitleTrack {
                    lang: lang.to_string(),
                    name: format!("{} (Auto)", lang),
                    is_auto: true,
                });
            }
        }
    }
    tracks
}

fn extract_chapters(v: &Value) -> Vec<ChapterItem> {
    let mut list = Vec::new();
    if let Some(chapters) = v.get("chapters").and_then(|c| c.as_array()) {
        for ch in chapters {
            let title = ch
                .get("title")
                .and_then(|s| s.as_str())
                .unwrap_or("Chapter")
                .to_string();
            let start_time = ch.get("start_time").and_then(|n| n.as_f64()).unwrap_or(0.0);
            let end_time = ch.get("end_time").and_then(|n| n.as_f64()).unwrap_or(0.0);
            list.push(ChapterItem {
                title,
                start_time,
                end_time,
            });
        }
    }
    list
}

/// Parses Instagram's Polaris GraphQL response (`xig_polaris_media`, `if_not_gated_logged_out`, or `xdt_shortcode_media`)
/// extracted via `yt-dlp --write-pages --skip-download --playlist-items 1` for public Instagram photos and 1080p carousels.
pub fn parse_instagram_graphql_dump_json(
    raw_json: &str,
    fallback_shortcode: Option<&str>,
) -> AppResult<MediaInspectionResult> {
    let v: Value = serde_json::from_str(raw_json).map_err(|e| {
        AppError::Internal(format!("Failed to parse Instagram GraphQL JSON: {}", e))
    })?;

    let media = v
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
        .or_else(|| {
            v.pointer("/graphql/shortcode_media")
                .filter(|x| !x.is_null() && x.is_object())
        })
        .ok_or_else(|| {
            AppError::AuthenticationRequired(
                "Instagram GraphQL returned gated or empty media (post may be private or require authentication)."
                    .into(),
            )
        })?;

    let shortcode = media
        .get("shortcode")
        .or_else(|| media.get("code"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .or_else(|| fallback_shortcode.map(|s| s.to_string()));

    let author_username = media
        .pointer("/user/username")
        .or_else(|| media.pointer("/owner/username"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("instagram_user")
        .to_string();

    let author_full_name = media
        .pointer("/user/full_name")
        .or_else(|| media.pointer("/owner/full_name"))
        .and_then(|s| s.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let caption = media
        .pointer("/caption/text")
        .or_else(|| media.pointer("/edge_media_to_caption/edges/0/node/text"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());

    let timestamp = media
        .get("taken_at")
        .or_else(|| media.get("taken_at_timestamp"))
        .and_then(|n| n.as_i64());

    let like_count = media
        .get("like_count")
        .or_else(|| media.pointer("/edge_media_preview_like/count"))
        .or_else(|| media.pointer("/edge_liked_by/count"))
        .and_then(|n| n.as_u64());

    let comment_count = media
        .get("comment_count")
        .or_else(|| media.pointer("/edge_media_to_comment/count"))
        .and_then(|n| n.as_u64());

    let mut slides = Vec::new();

    if let Some(carousel) = media.get("carousel_media").and_then(|c| c.as_array()) {
        for (idx, node) in carousel.iter().enumerate() {
            if let Some(slide) = extract_slide_from_graphql_node(node, idx + 1) {
                slides.push(slide);
            }
        }
    } else if let Some(edges) = media
        .pointer("/edge_sidecar_to_children/edges")
        .and_then(|e| e.as_array())
    {
        for (idx, edge) in edges.iter().enumerate() {
            let node = edge.get("node").unwrap_or(edge);
            if let Some(slide) = extract_slide_from_graphql_node(node, idx + 1) {
                slides.push(slide);
            }
        }
    }

    if slides.is_empty() {
        if let Some(slide) = extract_slide_from_graphql_node(media, 1) {
            slides.push(slide);
        }
    }

    if slides.is_empty() {
        return Err(AppError::MediaUnavailable(
            "No downloadable image or video URLs found in Instagram GraphQL response.".into(),
        ));
    }

    let source_url = match shortcode.as_deref() {
        Some(code) => format!("https://www.instagram.com/p/{}/", code),
        None => "https://www.instagram.com/".to_string(),
    };

    let url_type = if slides.len() > 1 {
        "carousel".to_string()
    } else if slides[0].media_type == SlideMediaType::Video {
        "reel".to_string()
    } else {
        "post".to_string()
    };

    Ok(MediaInspectionResult {
        source_url,
        platform: Platform::Instagram,
        shortcode,
        url_type,
        author_username,
        author_full_name,
        caption,
        timestamp,
        like_count,
        view_count: None,
        comment_count,
        slide_count: slides.len(),
        slides,
        available_qualities: vec![
            "best".into(),
            "1080p".into(),
            "720p".into(),
            "audio_mp3".into(),
            "audio_m4a".into(),
        ],
        formats: Vec::new(),
        subtitles: Vec::new(),
        chapters: Vec::new(),
        is_playlist: false,
        playlist_title: None,
        playlist_count: None,
        playlist_entries: Vec::new(),
        engine_used: "instagram-graphql".into(),
    })
}

fn extract_best_graphql_image_url(node: &Value) -> Option<String> {
    if let Some(candidates) = node
        .pointer("/image_versions2/candidates")
        .and_then(|c| c.as_array())
    {
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
            return Some(url.to_string());
        }

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
                    return Some(u.to_string());
                }
            }
        }
        if let Some(first_url) = candidates
            .first()
            .and_then(|c| c.get("url"))
            .and_then(|x| x.as_str())
        {
            return Some(first_url.to_string());
        }
    }

    if let Some(resources) = node.get("display_resources").and_then(|r| r.as_array()) {
        if let Some(best) = resources
            .iter()
            .max_by_key(|r| r.get("config_width").and_then(|w| w.as_u64()).unwrap_or(0))
        {
            if let Some(src) = best.get("src").and_then(|s| s.as_str()) {
                return Some(src.to_string());
            }
        }
    }

    node.get("display_uri")
        .or_else(|| node.get("display_url"))
        .or_else(|| node.get("thumbnail_src"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn extract_slide_from_graphql_node(node: &Value, index: usize) -> Option<MediaSlide> {
    let media_type_num = node.get("media_type").and_then(|x| x.as_u64()).unwrap_or(1);
    let typename = node
        .get("__typename")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    let is_video_flag = node
        .get("is_video")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);

    let video_url = node
        .get("video_versions")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|item| item.get("url"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            node.get("video_url")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
        });

    let is_video = media_type_num == 2
        || typename.contains("Video")
        || is_video_flag
        || video_url.is_some();

    let display_url = extract_best_graphql_image_url(node);

    let width = node
        .get("original_width")
        .or_else(|| node.pointer("/dimensions/width"))
        .and_then(|n| n.as_u64())
        .map(|n| n as u32)
        .or(Some(1080));
    let height = node
        .get("original_height")
        .or_else(|| node.pointer("/dimensions/height"))
        .and_then(|n| n.as_u64())
        .map(|n| n as u32)
        .or(Some(if is_video { 1920 } else { 1080 }));

    let direct_url = if is_video {
        video_url.or_else(|| display_url.clone())
    } else {
        display_url.clone()
    };

    let duration_seconds = node.get("video_duration").and_then(|n| n.as_f64());

    if direct_url.is_none() && display_url.is_none() {
        return None;
    }

    Some(MediaSlide {
        index,
        media_type: if is_video {
            SlideMediaType::Video
        } else {
            SlideMediaType::Image
        },
        thumbnail_url: display_url,
        direct_url,
        width,
        height,
        duration_seconds,
        filesize_approx: None,
    })
}

/// Parses `gallery-dl --dump-json` output for Instagram Carousels/Stories/Highlights and Facebook Photo Albums.
pub fn parse_gallery_dl_inspect_json(
    raw_output: &str,
    canonical_url: &str,
    fallback_shortcode: Option<&str>,
    platform: Platform,
) -> AppResult<MediaInspectionResult> {
    let v: Value = serde_json::from_str(raw_output).map_err(|e| {
        AppError::Internal(format!("Failed to parse gallery-dl JSON output: {}", e))
    })?;

    let items = v.as_array().ok_or_else(|| {
        AppError::Internal("Expected top-level JSON array from gallery-dl.".into())
    })?;

    let mut author_username = "creator".to_string();
    let mut author_full_name: Option<String> = None;
    let mut caption: Option<String> = None;
    let mut shortcode: Option<String> = fallback_shortcode.map(|s| s.to_string());
    let mut like_count: Option<u64> = None;
    let mut slides: Vec<MediaSlide> = Vec::new();

    for entry in items {
        let Some(tuple) = entry.as_array() else {
            continue;
        };
        if tuple.is_empty() {
            continue;
        }
        let tag = tuple[0].as_i64().unwrap_or(0);

        if tag == -1 {
            if let Some(err_obj) = tuple.get(1) {
                let msg = err_obj
                    .get("message")
                    .or_else(|| err_obj.get("error"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("gallery-dl extraction error");
                return Err(AppError::classify_engine_stderr(msg));
            }
        } else if tag == 2 {
            if let Some(meta) = tuple.get(1) {
                extract_gallery_dl_metadata(
                    meta,
                    &mut author_username,
                    &mut author_full_name,
                    &mut caption,
                    &mut shortcode,
                    &mut like_count,
                );
            }
        } else if tag == 3 {
            let direct_url = tuple
                .get(1)
                .and_then(|u| u.as_str())
                .map(|s| s.to_string());
            let meta = tuple.get(2).unwrap_or(&Value::Null);

            extract_gallery_dl_metadata(
                meta,
                &mut author_username,
                &mut author_full_name,
                &mut caption,
                &mut shortcode,
                &mut like_count,
            );

            let ext = meta
                .get("extension")
                .and_then(|s| s.as_str())
                .unwrap_or("jpg")
                .to_lowercase();
            let has_video_url = meta
                .get("video_url")
                .and_then(|s| s.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false);

            let media_type = if matches!(ext.as_str(), "mp4" | "webm" | "mov") || has_video_url {
                SlideMediaType::Video
            } else {
                SlideMediaType::Image
            };

            let thumb = meta
                .get("display_url")
                .or_else(|| meta.get("thumbnail"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .or_else(|| direct_url.clone());

            let width = meta.get("width").and_then(|n| n.as_u64()).map(|n| n as u32);
            let height = meta
                .get("height")
                .and_then(|n| n.as_u64())
                .map(|n| n as u32);

            slides.push(MediaSlide {
                index: slides.len() + 1,
                media_type,
                thumbnail_url: thumb,
                direct_url,
                width,
                height,
                duration_seconds: None,
                filesize_approx: None,
            });
        }
    }

    if slides.is_empty() {
        return Err(AppError::MediaUnavailable(
            "No downloadable media items were returned by gallery-dl.".into(),
        ));
    }

    let url_type = if slides.len() > 1 {
        "carousel".to_string()
    } else if slides[0].media_type == SlideMediaType::Video {
        "video".to_string()
    } else {
        "photo".to_string()
    };

    Ok(MediaInspectionResult {
        source_url: canonical_url.to_string(),
        platform,
        shortcode,
        url_type,
        author_username,
        author_full_name,
        caption,
        timestamp: None,
        like_count,
        view_count: None,
        comment_count: None,
        slide_count: slides.len(),
        slides,
        available_qualities: vec![
            "best".into(),
            "1080p".into(),
            "720p".into(),
            "audio_mp3".into(),
            "audio_m4a".into(),
        ],
        formats: Vec::new(),
        subtitles: Vec::new(),
        chapters: Vec::new(),
        is_playlist: false,
        playlist_title: None,
        playlist_count: None,
        playlist_entries: Vec::new(),
        engine_used: "gallery-dl".into(),
    })
}

fn extract_gallery_dl_metadata(
    meta: &Value,
    author_username: &mut String,
    author_full_name: &mut Option<String>,
    caption: &mut Option<String>,
    shortcode: &mut Option<String>,
    like_count: &mut Option<u64>,
) {
    if let Some(u) = meta
        .get("username")
        .or_else(|| meta.get("author"))
        .or_else(|| meta.pointer("/owner/username"))
        .and_then(|s| s.as_str())
    {
        if !u.is_empty() {
            *author_username = u.to_string();
        }
    }
    if author_full_name.is_none() {
        *author_full_name = meta
            .get("fullname")
            .or_else(|| meta.pointer("/owner/full_name"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
    }
    if caption.is_none() {
        *caption = meta
            .get("description")
            .or_else(|| meta.get("text"))
            .or_else(|| meta.get("content"))
            .and_then(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
    }
    if shortcode.is_none() {
        *shortcode = meta
            .get("post_shortcode")
            .or_else(|| meta.get("shortcode"))
            .or_else(|| meta.get("post_id"))
            .or_else(|| meta.get("id"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string());
    }
    if like_count.is_none() {
        *like_count = meta
            .get("likes")
            .or_else(|| meta.get("reactions"))
            .and_then(|n| n.as_u64());
    }
}

/// Parses a single stdout line from `yt-dlp --newline --progress-template "download:DL_PROGRESS|..."`
pub fn parse_ytdlp_stdout_line(
    job_id: &str,
    slide_index: usize,
    line: &str,
) -> Option<ProgressPayload> {
    let trimmed = line.trim();

    if trimmed.contains("[Merger]")
        || trimmed.contains("[FixupM3u8]")
        || trimmed.contains("Merging formats into")
    {
        return Some(ProgressPayload {
            job_id: job_id.to_string(),
            slide_index,
            status: "muxing".into(),
            downloaded_bytes: 0,
            total_bytes: None,
            speed_bps: None,
            eta_seconds: Some(1),
            percentage: 99.0,
        });
    }

    if trimmed.contains("[ExtractAudio]") || trimmed.contains("[ModifyChapters]") {
        return Some(ProgressPayload {
            job_id: job_id.to_string(),
            slide_index,
            status: "extracting_audio".into(),
            downloaded_bytes: 0,
            total_bytes: None,
            speed_bps: None,
            eta_seconds: Some(1),
            percentage: 99.0,
        });
    }

    if let Some(rest) = trimmed.strip_prefix("DL_PROGRESS|") {
        let parts: Vec<&str> = rest.split('|').collect();
        if parts.len() >= 5 {
            let downloaded_bytes = parts[0].trim().parse::<u64>().unwrap_or(0);
            let total_bytes = parts[1].trim().parse::<u64>().ok();
            let eta_seconds = parts[2].trim().parse::<f64>().ok().map(|v| v as u64);
            let speed_bps = parts[3].trim().parse::<f64>().ok();

            let pct_clean = parts[4].trim().trim_end_matches('%').trim();
            let percentage = pct_clean.parse::<f64>().unwrap_or_else(|_| {
                if let Some(tot) = total_bytes {
                    if tot > 0 {
                        (downloaded_bytes as f64 / tot as f64) * 100.0
                    } else {
                        0.0
                    }
                } else {
                    0.0
                }
            });

            return Some(ProgressPayload {
                job_id: job_id.to_string(),
                slide_index,
                status: "downloading".into(),
                downloaded_bytes,
                total_bytes,
                speed_bps,
                eta_seconds,
                percentage: percentage.clamp(0.0, 100.0),
            });
        }
    }

    if trimmed.starts_with("[download]") && trimmed.contains('%') {
        for token in trimmed.split_whitespace() {
            if let Some(num_str) = token.strip_suffix('%') {
                if let Ok(pct) = num_str.parse::<f64>() {
                    return Some(ProgressPayload {
                        job_id: job_id.to_string(),
                        slide_index,
                        status: "downloading".into(),
                        downloaded_bytes: 0,
                        total_bytes: None,
                        speed_bps: None,
                        eta_seconds: None,
                        percentage: pct.clamp(0.0, 100.0),
                    });
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ytdlp_youtube_4k() {
        let sample = r#"{
            "id": "dQw4w9WgXcQ",
            "extractor_key": "Youtube",
            "webpage_url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
            "uploader": "Rick Astley",
            "title": "Never Gonna Give You Up",
            "duration": 212.0,
            "formats": [
                {"format_id": "137", "height": 1080, "width": 1920, "fps": 60.0, "vcodec": "avc1", "acodec": "none"},
                {"format_id": "313", "height": 2160, "width": 3840, "fps": 30.0, "vcodec": "vp9", "acodec": "none"}
            ]
        }"#;
        let res = parse_ytdlp_inspect_json(sample, None, None).unwrap();
        assert_eq!(res.platform, Platform::YouTube);
        assert!(res.available_qualities.contains(&"4k".to_string()));
        assert!(res.available_qualities.contains(&"1080p60".to_string()));
    }

    #[test]
    fn test_parse_instagram_graphql_carousel() {
        let sample = r#"{
            "data": {
                "xig_polaris_media": {
                    "__typename": "XDTGraphSidecar",
                    "shortcode": "DT8abc123",
                    "owner": {"username": "natgeo", "full_name": "National Geographic"},
                    "edge_sidecar_to_children": {
                        "edges": [
                            {"node": {"__typename": "XDTGraphImage", "is_video": false, "display_url": "https://cdn.ig/1.jpg", "dimensions": {"width": 1080, "height": 1350}}},
                            {"node": {"__typename": "XDTGraphVideo", "is_video": true, "display_url": "https://cdn.ig/2.jpg", "video_url": "https://cdn.ig/2.mp4", "dimensions": {"width": 1080, "height": 1920}}}
                        ]
                    }
                }
            }
        }"#;
        let res = parse_instagram_graphql_dump_json(sample, None).unwrap();
        assert_eq!(res.platform, Platform::Instagram);
        assert_eq!(res.slide_count, 2);
        assert_eq!(res.slides[0].media_type, SlideMediaType::Image);
        assert_eq!(res.slides[1].media_type, SlideMediaType::Video);
    }
}
