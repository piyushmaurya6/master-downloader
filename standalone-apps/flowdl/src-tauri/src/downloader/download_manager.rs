use crate::models::{AppSettings, DownloadRequest};
use crate::services::metadata_service::validate_media_url;
use crate::utils::find_bundled_deno_path;
use std::path::{Path, PathBuf};

/// Builds a structured argument list for yt-dlp. Never uses shell interpolation.
pub fn build_ytdlp_arguments(
    req: &DownloadRequest,
    settings: &AppSettings,
    ffmpeg_dir: Option<&Path>,
) -> Result<(Vec<String>, String, String), String> {
    let clean_url = validate_media_url(&req.url)?;
    let mut args: Vec<String> = Vec::new();

    // Machine-readable progress output
    args.push("--newline".to_string());
    args.push("--progress".to_string());
    args.push("--no-colors".to_string());
    args.push("--progress-template".to_string());
    args.push(
        "download:FLOWDL_PROGRESS|%(progress._percent_str)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s"
            .to_string(),
    );
    args.push("--print".to_string());
    args.push("after_move:FLOWDL_FILEPATH|%(filepath)s".to_string());

    // Bundled JS runtime for YouTube challenge solving
    if let Some(deno_path) = find_bundled_deno_path() {
        args.push("--js-runtimes".to_string());
        args.push(format!("deno:{}", deno_path.to_string_lossy()));
    }

    // Resumable partial downloads
    args.push("--continue".to_string());

    if settings.ask_before_overwrite {
        args.push("--no-overwrites".to_string());
    }

    // Playlist handling
    if req.playlist_mode == "playlist" {
        args.push("--yes-playlist".to_string());
    } else {
        args.push("--no-playlist".to_string());
    }

    // FFmpeg location
    if let Some(dir) = ffmpeg_dir {
        args.push("--ffmpeg-location".to_string());
        args.push(dir.to_string_lossy().to_string());
    }

    // Speed limit
    let speed_limit = req
        .speed_limit_kbps
        .unwrap_or(settings.speed_limit_kbps);
    if speed_limit > 0 {
        args.push("--limit-rate".to_string());
        args.push(format!("{}K", speed_limit));
    }

    // Output directory & naming template
    let base_dir = req
        .output_dir
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&settings.download_dir);

    let template = if req.preserve_original_filename {
        "%(title)s.%(ext)s".to_string()
    } else {
        req.filename_template
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| settings.filename_template.clone())
    };

    let full_output_template = if req.playlist_mode == "playlist" && settings.create_playlist_folders {
        PathBuf::from(base_dir)
            .join("%(playlist_title)s")
            .join(&template)
            .to_string_lossy()
            .to_string()
    } else {
        PathBuf::from(base_dir)
            .join(&template)
            .to_string_lossy()
            .to_string()
    };

    args.push("-o".to_string());
    args.push(full_output_template);

    let format_summary: String;
    let quality_label: String;

    if req.mode == "audio" {
        args.push("-x".to_string());
        let audio_fmt = req.audio_format.to_lowercase();
        let target_fmt = match audio_fmt.as_str() {
            "mp3" | "m4a" | "flac" | "wav" | "opus" => audio_fmt.as_str(),
            _ => "best",
        };
        args.push("--audio-format".to_string());
        args.push(target_fmt.to_string());

        args.push("--audio-quality".to_string());
        let q_arg = match req.audio_quality.as_str() {
            "320" => "320K",
            "256" => "256K",
            "192" => "192K",
            "128" => "128K",
            _ => "0", // best VBR/source
        };
        args.push(q_arg.to_string());

        if req.normalize_audio {
            args.push("--postprocessor-args".to_string());
            args.push("FFmpegExtractAudio:-af loudnorm=I=-16:TP=-1.5:LRA=11".to_string());
        }

        quality_label = if req.audio_quality == "best" {
            "Best Source".to_string()
        } else {
            format!("{} kbps", req.audio_quality)
        };
        format_summary = format!("{} · {}", target_fmt.to_uppercase(), quality_label);
    } else {
        // Video mode
        let height_num: Option<u32> = req.video_quality.parse::<u32>().ok();
        let height_filter = match height_num {
            Some(h) if req.exact_resolution => format!("[height={}]", h),
            Some(h) => format!("[height<={}]", h),
            None => String::new(),
        };

        let codec_filter = match req.video_codec.as_str() {
            "h264" => "[vcodec^=avc1]",
            "h265" => "[vcodec^=hev1]",
            "vp9" => "[vcodec^=vp9]",
            "av1" => "[vcodec^=av01]",
            _ => "",
        };

        let fps_filter = match req.preferred_fps {
            Some(fps) if fps > 0 => format!("[fps<={}]", fps),
            _ => String::new(),
        };

        let container = req.container.to_lowercase();
        let format_expr = if !codec_filter.is_empty() {
            format!(
                "bestvideo{}{}{}+bestaudio/bestvideo{}{}+bestaudio/best{}",
                height_filter, codec_filter, fps_filter, height_filter, fps_filter, height_filter
            )
        } else if container == "mp4" {
            format!(
                "bestvideo{}[ext=mp4]{}+bestaudio[ext=m4a]/bestvideo{}{}+bestaudio/best{}",
                height_filter, fps_filter, height_filter, fps_filter, height_filter
            )
        } else if container == "webm" {
            format!(
                "bestvideo{}[ext=webm]{}+bestaudio[ext=webm]/bestvideo{}{}+bestaudio/best{}",
                height_filter, fps_filter, height_filter, fps_filter, height_filter
            )
        } else {
            format!(
                "bestvideo{}{}+bestaudio/best{}",
                height_filter, fps_filter, height_filter
            )
        };

        args.push("-f".to_string());
        args.push(format_expr);

        if container == "mp4" || container == "mkv" || container == "webm" {
            args.push("--merge-output-format".to_string());
            args.push(container.clone());
        }

        if req.download_subtitles {
            args.push("--write-subs".to_string());
            args.push("--write-auto-subs".to_string());
            let lang = if req.subtitle_lang.trim().is_empty() {
                "en"
            } else {
                req.subtitle_lang.trim()
            };
            args.push("--sub-langs".to_string());
            args.push(lang.to_string());
        }

        quality_label = match height_num {
            Some(2160) => "4K (2160p)".to_string(),
            Some(1440) => "1440p".to_string(),
            Some(h) => format!("{}p", h),
            None => "Best".to_string(),
        };
        let cont_label = if container == "auto" {
            "Auto".to_string()
        } else {
            container.to_uppercase()
        };
        format_summary = format!("{} · {}", quality_label, cont_label);
    }

    // Metadata & Thumbnails
    if req.embed_metadata {
        args.push("--embed-metadata".to_string());
    }
    if req.embed_thumbnail {
        // WAV does not reliably support embedded thumbnails via yt-dlp without erroring
        if !(req.mode == "audio" && req.audio_format.eq_ignore_ascii_case("wav")) {
            args.push("--embed-thumbnail".to_string());
        }
    }
    if req.download_chapters {
        args.push("--embed-chapters".to_string());
    }
    if settings.preserve_intermediate_files {
        args.push("--keep-video".to_string());
    }

    // Always separate flags from the URL with "--" to prevent argument injection
    args.push("--".to_string());
    args.push(clean_url);

    Ok((args, format_summary, quality_label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_video_and_audio_arguments_safely() {
        let settings = AppSettings::default();
        let req = DownloadRequest {
            url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
            title: "Test Video".to_string(),
            thumbnail: None,
            uploader: None,
            duration: Some(120.0),
            extractor: "Youtube".to_string(),
            mode: "video".to_string(),
            video_quality: "1080".to_string(),
            exact_resolution: false,
            container: "mp4".to_string(),
            video_codec: "h264".to_string(),
            preferred_fps: Some(60),
            audio_format: "mp3".to_string(),
            audio_quality: "320".to_string(),
            download_subtitles: true,
            subtitle_lang: "en".to_string(),
            embed_thumbnail: true,
            embed_metadata: true,
            download_chapters: true,
            normalize_audio: false,
            preserve_original_filename: false,
            filename_template: None,
            speed_limit_kbps: Some(5000),
            playlist_mode: "single".to_string(),
            output_dir: Some("C:\\Users\\Test\\My Videos With Spaces".to_string()),
        };

        let (args, summary, quality) =
            build_ytdlp_arguments(&req, &settings, Some(Path::new("C:\\FlowDL\\bin"))).unwrap();
        assert_eq!(quality, "1080p");
        assert_eq!(summary, "1080p · MP4");
        assert!(args.contains(&"--merge-output-format".to_string()));
        assert!(args.contains(&"mp4".to_string()));
        assert!(args.contains(&"--".to_string()));
        assert_eq!(args.last().unwrap(), "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    }
}
