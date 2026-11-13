use crate::core::auth_vault::{AuthVault, EphemeralCookieGuard};
use crate::core::binary_manager::{new_hidden_tokio_command, BinaryManager, ProcessRegistry};
use crate::core::parser::{
    is_circuit_breaker_trigger, parse_gallery_dl_inspect_json, parse_ytdlp_inspect_json,
    parse_ytdlp_stdout_line, JobStatus, MediaInspectionResult, ProgressPayload,
    ThrottledProgressEmitter,
};
use crate::core::url_normalizer::{
    normalize_facebook_url, resolve_output_path, FacebookUrlType, NormalizedFacebookUrl,
};
use crate::db::{AppSettings, JobRecord, MediaFileRecord, QualityPreset};
use crate::error::{AppError, AppResult};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tauri::AppHandle;
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

#[allow(async_fn_in_trait)]
pub trait MediaEngine {
    async fn inspect(
        &self,
        norm_url: &NormalizedFacebookUrl,
        settings: &AppSettings,
    ) -> AppResult<MediaInspectionResult>;

    async fn download_job(
        &self,
        job: &JobRecord,
        settings: &AppSettings,
        app: Option<&AppHandle>,
    ) -> AppResult<Vec<MediaFileRecord>>;
}

#[derive(Clone)]
pub struct EngineRouter {
    binary_manager: BinaryManager,
    auth_vault: AuthVault,
    process_registry: ProcessRegistry,
    cache_dir: PathBuf,
}

impl EngineRouter {
    pub fn new(
        binary_manager: BinaryManager,
        auth_vault: AuthVault,
        process_registry: ProcessRegistry,
        cache_dir: PathBuf,
    ) -> Self {
        let _ = std::fs::create_dir_all(&cache_dir);
        Self {
            binary_manager,
            auth_vault,
            process_registry,
            cache_dir,
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Appends authentication (`--cookies` or `--cookies-from-browser`) and proxy flags to a CLI argument list.
    fn append_auth_and_network_args(
        &self,
        args: &mut Vec<String>,
        settings: &AppSettings,
    ) -> AppResult<Option<EphemeralCookieGuard>> {
        if let Some(ref proxy) = settings.proxy_url {
            let trimmed = proxy.trim();
            if !trimmed.is_empty() {
                args.push("--proxy".to_string());
                args.push(trimmed.to_string());
            }
        }

        match settings.auth_mode.as_str() {
            "webview_vault" | "cookie_file" => {
                if self.auth_vault.has_encrypted_vault() {
                    let guard = self.auth_vault.create_ephemeral_cookie_guard()?;
                    args.push("--cookies".to_string());
                    args.push(guard.path().to_string_lossy().to_string());
                    return Ok(Some(guard));
                }
            }
            "browser_cookies" => {
                if let Some(ref browser) = settings.selected_browser {
                    let b = browser.trim().to_lowercase();
                    if matches!(b.as_str(), "edge" | "chrome" | "firefox" | "brave") {
                        args.push("--cookies-from-browser".to_string());
                        args.push(b);
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    /// Resolves `fb.watch/*` and `facebook.com/share/*` shortlinks via pre-flight HTTP redirect inspection
    /// so canonical IDs and URL types (`REEL` vs `WATCH` vs `POST`) are accurately discovered.
    pub async fn resolve_redirect_if_needed(
        &self,
        norm: &NormalizedFacebookUrl,
    ) -> NormalizedFacebookUrl {
        let needs_redirect = matches!(
            norm.url_type,
            FacebookUrlType::Share { .. } | FacebookUrlType::Shortlink(_)
        );
        if !needs_redirect {
            return norm.clone();
        }

        let Ok(client) = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(8))
            .timeout(Duration::from_secs(10))
            .build()
        else {
            return norm.clone();
        };

        if let Ok(resp) = client.get(&norm.canonical_url).send().await {
            let final_url = resp.url().as_str().to_string();
            if !final_url.contains("/login") {
                if let Ok(resolved) = normalize_facebook_url(&final_url) {
                    return resolved;
                }
            }
        }

        norm.clone()
    }

    pub async fn analyze_url(
        &self,
        raw_url: &str,
        settings: &AppSettings,
    ) -> AppResult<MediaInspectionResult> {
        let initial_norm = normalize_facebook_url(raw_url)?;
        let resolved_norm = self.resolve_redirect_if_needed(&initial_norm).await;
        self.inspect(&resolved_norm, settings).await
    }

    async fn run_ytdlp_inspect(
        &self,
        norm: &NormalizedFacebookUrl,
        settings: &AppSettings,
    ) -> AppResult<MediaInspectionResult> {
        let bin = self.binary_manager.yt_dlp_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing("yt-dlp.exe".to_string()));
        }

        let mut args = vec![
            "--dump-single-json".to_string(),
            "--no-warnings".to_string(),
            "--no-playlist".to_string(),
            "--socket-timeout".to_string(),
            "25".to_string(),
        ];

        let _cookie_guard = self.append_auth_and_network_args(&mut args, settings)?;
        args.push("--".to_string());
        args.push(norm.canonical_url.clone());

        let mut cmd = new_hidden_tokio_command(&bin);
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn()?;
        let pid = child.id();
        if let Some(p) = pid {
            self.process_registry.register("inspect", p);
        }

        let output = tokio::time::timeout(Duration::from_secs(45), child.wait_with_output())
            .await
            .map_err(|_| AppError::NetworkOffline)??;

        if let Some(p) = pid {
            self.process_registry.unregister("inspect", p);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && stdout.trim().is_empty() {
            return Err(AppError::from_engine_stderr(&stderr));
        }

        parse_ytdlp_inspect_json(&stdout, norm)
    }

    async fn run_gallery_dl_inspect(
        &self,
        norm: &NormalizedFacebookUrl,
        settings: &AppSettings,
    ) -> AppResult<MediaInspectionResult> {
        let bin = self.binary_manager.gallery_dl_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing("gallery-dl.exe".to_string()));
        }

        let mut args = vec![
            "--dump-json".to_string(),
            "--no-colors".to_string(),
            "--retries".to_string(),
            "2".to_string(),
        ];

        let _cookie_guard = self.append_auth_and_network_args(&mut args, settings)?;
        args.push("--".to_string());
        args.push(norm.canonical_url.clone());

        let mut cmd = new_hidden_tokio_command(&bin);
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn()?;
        let pid = child.id();
        if let Some(p) = pid {
            self.process_registry.register("inspect", p);
        }

        let output = tokio::time::timeout(Duration::from_secs(45), child.wait_with_output())
            .await
            .map_err(|_| AppError::NetworkOffline)??;

        if let Some(p) = pid {
            self.process_registry.unregister("inspect", p);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && stdout.trim().is_empty() {
            return Err(AppError::from_engine_stderr(&stderr));
        }

        parse_gallery_dl_inspect_json(&stdout, norm)
    }

    pub fn get_cached_inspection(&self, media_id: &str) -> Option<MediaInspectionResult> {
        let path = self
            .cache_dir
            .join("inspections")
            .join(format!("{}.json", media_id));
        let meta = std::fs::metadata(&path).ok()?;
        let modified = meta.modified().ok()?;
        if modified.elapsed().unwrap_or(Duration::from_secs(3600)) > Duration::from_secs(900) {
            return None;
        }
        let raw = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str::<MediaInspectionResult>(&raw).ok()
    }

    fn save_cached_inspection(&self, media_id: &str, result: &MediaInspectionResult) {
        let dir = self.cache_dir.join("inspections");
        if std::fs::create_dir_all(&dir).is_ok() {
            if let Ok(json) = serde_json::to_string(result) {
                let _ = std::fs::write(dir.join(format!("{}.json", media_id)), json);
            }
        }
    }

    /// Proxies a Facebook CDN (`fbcdn.net`) thumbnail image through Rust with proper `Referer` and `User-Agent` headers,
    /// caching the image on disk and returning a `data:image/jpeg;base64,...` URI so WebView CORS policies never block previews.
    pub async fn proxy_thumbnail(&self, remote_url: &str) -> AppResult<String> {
        let trimmed = remote_url.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidUrl("Empty thumbnail URL".to_string()));
        }
        if trimmed.starts_with("data:") {
            return Ok(trimmed.to_string());
        }

        let mut hasher = Sha256::new();
        hasher.update(trimmed.as_bytes());
        let hash_hex = hex::encode(hasher.finalize());
        let thumbs_dir = self.cache_dir.join("thumbs");
        std::fs::create_dir_all(&thumbs_dir)?;
        let cached_file = thumbs_dir.join(format!("{}.jpg", hash_hex));

        if cached_file.exists() {
            if let Ok(bytes) = std::fs::read(&cached_file) {
                if !bytes.is_empty() {
                    return Ok(format!("data:image/jpeg;base64,{}", B64.encode(&bytes)));
                }
            }
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(15))
            .build()?;

        let resp = client
            .get(trimmed)
            .header("Referer", "https://www.facebook.com/")
            .header("Accept", "image/avif,image/webp,image/apng,image/*,*/*;q=0.8")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::ProcessError(format!(
                "Thumbnail HTTP {}",
                resp.status()
            )));
        }

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let bytes = resp.bytes().await?;
        let _ = std::fs::write(&cached_file, &bytes);

        Ok(format!(
            "data:{};base64,{}",
            content_type,
            B64.encode(&bytes)
        ))
    }

    pub fn build_ytdlp_format_selector(preset: &QualityPreset, raw_preset_str: &str) -> String {
        match preset {
            QualityPreset::Best => {
                "bestvideo[ext=mp4]+bestaudio[ext=m4a]/bestvideo+bestaudio/best[ext=mp4]/hd/best"
                    .to_string()
            }
            QualityPreset::Mp4_4k => {
                "bestvideo[height<=2160][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=2160]+bestaudio/best[height<=2160]/hd/best"
                    .to_string()
            }
            QualityPreset::Mp4_1440p => {
                "bestvideo[height<=1440][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=1440]+bestaudio/best[height<=1440]/hd/best"
                    .to_string()
            }
            QualityPreset::Mp4_1080p => {
                "bestvideo[height<=1080][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=1080]+bestaudio/best[height<=1080]/hd/best"
                    .to_string()
            }
            QualityPreset::Mp4_720p => {
                "bestvideo[height<=720][ext=mp4]+bestaudio[ext=m4a]/bestvideo[height<=720]+bestaudio/hd/best[height<=720]/best"
                    .to_string()
            }
            QualityPreset::Mp4_480p => {
                "bestvideo[height<=480][ext=mp4]+bestaudio[ext=m4a]/sd/best[height<=480]/best"
                    .to_string()
            }
            QualityPreset::AudioMp3 => "bestaudio/best".to_string(),
            QualityPreset::AudioM4a => "bestaudio[ext=m4a]/bestaudio/best".to_string(),
            QualityPreset::CustomFormat => {
                if raw_preset_str.contains('+') || raw_preset_str.contains('/') {
                    raw_preset_str.to_string()
                } else {
                    format!("{}+bestaudio[ext=m4a]/{}+bestaudio/{}/best", raw_preset_str, raw_preset_str, raw_preset_str)
                }
            }
        }
    }
}

impl MediaEngine for EngineRouter {
    async fn inspect(
        &self,
        norm: &NormalizedFacebookUrl,
        settings: &AppSettings,
    ) -> AppResult<MediaInspectionResult> {
        if let Some(ref mid) = norm.shortcode {
            if let Some(cached) = self.get_cached_inspection(mid) {
                return Ok(cached);
            }
        }

        let res = match norm.url_type {
            FacebookUrlType::Photo(_) => {
                match self.run_gallery_dl_inspect(norm, settings).await {
                    Ok(res) => Ok(res),
                    Err(primary_err) => self
                        .run_ytdlp_inspect(norm, settings)
                        .await
                        .map_err(|_| primary_err),
                }
            }
            _ => match self.run_ytdlp_inspect(norm, settings).await {
                Ok(res) => Ok(res),
                Err(primary_err) => {
                    if let Ok(fallback) = self.run_gallery_dl_inspect(norm, settings).await {
                        Ok(fallback)
                    } else {
                        Err(primary_err)
                    }
                }
            },
        }?;

        if let Some(ref mid) = norm.shortcode.as_ref().or(res.shortcode.as_ref()) {
            self.save_cached_inspection(mid, &res);
        }

        Ok(res)
    }

    async fn download_job(
        &self,
        job: &JobRecord,
        settings: &AppSettings,
        app: Option<&AppHandle>,
    ) -> AppResult<Vec<MediaFileRecord>> {
        let initial_norm = normalize_facebook_url(&job.source_url)?;
        let norm = self.resolve_redirect_if_needed(&initial_norm).await;
        let preset = QualityPreset::from_str_lossy(&job.quality_preset);
        let base_dir = PathBuf::from(&job.output_dir);
        std::fs::create_dir_all(&base_dir)?;

        let date_str = Utc::now().format("%Y-%m-%d").to_string();
        let mut emitter = ThrottledProgressEmitter::new();

        emitter.emit_if_ready(
            app,
            &ProgressPayload {
                job_id: job.id.clone(),
                item_index: 1,
                total_items: job.selected_indices.len().max(1),
                status: JobStatus::Inspecting,
                stage_label: "Inspecting Facebook DASH streams...".to_string(),
                percentage: 2.0,
                downloaded_bytes: 0,
                total_bytes: None,
                speed_bps: None,
                eta_seconds: None,
            },
            true,
        );

        let inspection = self.inspect(&norm, settings).await.ok();

        let is_single_video = inspection
            .as_ref()
            .map(|i| {
                i.slides.len() <= 1
                    && i.slides
                        .first()
                        .map(|s| s.media_type == "video")
                        .unwrap_or(true)
            })
            .unwrap_or(!matches!(norm.url_type, FacebookUrlType::Photo(_)));

        if is_single_video || preset.is_audio_only() {
            if !self.binary_manager.ffmpeg_path().exists() {
                return Err(AppError::BinaryMissing("ffmpeg.exe".to_string()));
            }
        }

        let mut downloaded_files = Vec::new();

        if is_single_video {
            let ext = preset.output_extension();
            let target_path = resolve_output_path(
                &base_dir,
                &job.naming_template,
                job.author_username
                    .as_deref()
                    .or_else(|| inspection.as_ref().map(|i| i.owner_username.as_str())),
                job.shortcode
                    .as_deref()
                    .or_else(|| norm.shortcode.as_deref()),
                &date_str,
                1,
                ext,
            );
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let ytdlp_res = self
                .run_ytdlp_download_single(
                    job,
                    &norm.canonical_url,
                    &target_path,
                    &preset,
                    None,
                    1,
                    1,
                    settings,
                    app,
                    &mut emitter,
                )
                .await;

            if ytdlp_res.is_err() || !target_path.exists() {
                if let Some(v_url) = inspection
                    .as_ref()
                    .and_then(|i| i.slides.first())
                    .and_then(|s| s.video_url.as_deref())
                {
                    self.download_direct_cdn_asset(v_url, &target_path).await?;
                } else {
                    ytdlp_res?;
                }
            }

            let size = std::fs::metadata(&target_path).ok().map(|m| m.len());
            if !target_path.exists() || size.unwrap_or(0) == 0 {
                return Err(AppError::MediaDeletedOrUnavailable);
            }
            downloaded_files.push(MediaFileRecord {
                id: Uuid::new_v4().to_string(),
                job_id: job.id.clone(),
                slide_index: 1,
                media_type: if preset.is_audio_only() {
                    "audio".to_string()
                } else {
                    "video".to_string()
                },
                file_path: target_path.to_string_lossy().to_string(),
                file_size_bytes: size,
                width: inspection
                    .as_ref()
                    .and_then(|i| i.slides.first())
                    .map(|s| s.width)
                    .or(Some(1080)),
                height: inspection
                    .as_ref()
                    .and_then(|i| i.slides.first())
                    .map(|s| s.height)
                    .or(Some(1920)),
                duration_seconds: inspection
                    .as_ref()
                    .and_then(|i| i.slides.first())
                    .and_then(|s| s.duration_seconds),
                file_exists: target_path.exists(),
            });
        } else {
            let indices: Vec<usize> = if !job.selected_indices.is_empty() {
                job.selected_indices.clone()
            } else if let Some(ref insp) = inspection {
                (1..=insp.slides.len().max(1)).collect()
            } else {
                vec![1]
            };

            let total_items = indices.len().max(1);

            for (pos, &slide_idx) in indices.iter().enumerate() {
                let item_number = pos + 1;
                let slide_meta = inspection
                    .as_ref()
                    .and_then(|insp| insp.slides.iter().find(|s| s.index == slide_idx));

                let is_video_slide = slide_meta
                    .map(|s| s.media_type == "video")
                    .unwrap_or(false);

                let ext = if is_video_slide {
                    preset.output_extension()
                } else {
                    slide_meta.map(|s| s.ext.as_str()).unwrap_or("jpg")
                };

                let target_path = resolve_output_path(
                    &base_dir,
                    &job.naming_template,
                    job.author_username
                        .as_deref()
                        .or_else(|| inspection.as_ref().map(|i| i.owner_username.as_str())),
                    job.shortcode
                        .as_deref()
                        .or_else(|| norm.shortcode.as_deref()),
                    &date_str,
                    slide_idx,
                    ext,
                );
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                emitter.emit_if_ready(
                    app,
                    &ProgressPayload {
                        job_id: job.id.clone(),
                        item_index: item_number,
                        total_items,
                        status: JobStatus::DownloadingVideo,
                        stage_label: format!(
                            "Downloading item {} of {} ({})...",
                            item_number,
                            total_items,
                            if is_video_slide { "VIDEO" } else { "PHOTO" }
                        ),
                        percentage: ((pos as f64 / total_items as f64) * 100.0).clamp(5.0, 95.0),
                        downloaded_bytes: 0,
                        total_bytes: None,
                        speed_bps: None,
                        eta_seconds: None,
                    },
                    true,
                );

                if is_video_slide {
                    if !self.binary_manager.ffmpeg_path().exists() {
                        return Err(AppError::BinaryMissing("ffmpeg.exe".to_string()));
                    }
                    let ytdlp_res = self
                        .run_ytdlp_download_single(
                            job,
                            &norm.canonical_url,
                            &target_path,
                            &preset,
                            Some(slide_idx),
                            item_number,
                            total_items,
                            settings,
                            app,
                            &mut emitter,
                        )
                        .await;

                    if ytdlp_res.is_err() || !target_path.exists() {
                        if let Some(v_url) = slide_meta.and_then(|s| s.video_url.as_deref()) {
                            self.download_direct_cdn_asset(v_url, &target_path).await?;
                        } else {
                            self.run_gallery_dl_slide(
                                &norm.canonical_url,
                                slide_idx,
                                &target_path,
                                &job.id,
                                settings,
                            )
                            .await?;
                        }
                    }
                } else {
                    let mut downloaded_ok = false;
                    if let Some(slide) = slide_meta {
                        if !slide.display_url.is_empty() {
                            if self
                                .download_direct_cdn_asset(&slide.display_url, &target_path)
                                .await
                                .is_ok()
                                && target_path.exists()
                            {
                                downloaded_ok = true;
                            }
                        }
                    }

                    if !downloaded_ok {
                        self.run_gallery_dl_slide(
                            &norm.canonical_url,
                            slide_idx,
                            &target_path,
                            &job.id,
                            settings,
                        )
                        .await?;
                    }
                }

                let size = std::fs::metadata(&target_path).ok().map(|m| m.len());
                if !target_path.exists() || size.unwrap_or(0) == 0 {
                    return Err(AppError::MediaDeletedOrUnavailable);
                }

                downloaded_files.push(MediaFileRecord {
                    id: Uuid::new_v4().to_string(),
                    job_id: job.id.clone(),
                    slide_index: slide_idx,
                    media_type: if is_video_slide {
                        "video".to_string()
                    } else {
                        "image".to_string()
                    },
                    file_path: target_path.to_string_lossy().to_string(),
                    file_size_bytes: size,
                    width: slide_meta.map(|s| s.width).or(Some(1080)),
                    height: slide_meta.map(|s| s.height).or(Some(1080)),
                    duration_seconds: slide_meta.and_then(|s| s.duration_seconds),
                    file_exists: target_path.exists(),
                });
            }
        }

        if job.save_caption {
            let cap_opt = job
                .caption
                .as_deref()
                .or_else(|| inspection.as_ref().map(|i| i.caption.as_str()));
            if let Some(cap) = cap_opt {
                if !cap.trim().is_empty() {
                    if let Some(first_file) = downloaded_files.first() {
                        let mut txt_path = PathBuf::from(&first_file.file_path);
                        txt_path.set_extension("txt");
                        let _ = std::fs::write(&txt_path, cap.as_bytes());
                    }
                }
            }
        }

        Ok(downloaded_files)
    }
}

impl EngineRouter {
    #[allow(clippy::too_many_arguments)]
    async fn run_ytdlp_download_single(
        &self,
        job: &JobRecord,
        canonical_url: &str,
        target_path: &Path,
        preset: &QualityPreset,
        playlist_item: Option<usize>,
        item_index: usize,
        total_items: usize,
        settings: &AppSettings,
        app: Option<&AppHandle>,
        emitter: &mut ThrottledProgressEmitter,
    ) -> AppResult<()> {
        let bin = self.binary_manager.yt_dlp_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing("yt-dlp.exe".to_string()));
        }

        let format_selector = Self::build_ytdlp_format_selector(preset, &job.quality_preset);

        let mut args = vec![
            "--ffmpeg-location".to_string(),
            self.binary_manager.bin_dir().to_string_lossy().to_string(),
            "-f".to_string(),
            format_selector,
            "--newline".to_string(),
            "--progress".to_string(),
            "--no-warnings".to_string(),
            "--embed-metadata".to_string(),
        ];

        match preset {
            QualityPreset::AudioMp3 => {
                args.push("--extract-audio".to_string());
                args.push("--audio-format".to_string());
                args.push("mp3".to_string());
                args.push("--audio-quality".to_string());
                args.push("320K".to_string());
            }
            QualityPreset::AudioM4a => {
                args.push("--extract-audio".to_string());
                args.push("--audio-format".to_string());
                args.push("m4a".to_string());
            }
            _ => {
                args.push("--merge-output-format".to_string());
                args.push("mp4".to_string());
            }
        }

        if let Some(idx) = playlist_item {
            args.push("--playlist-items".to_string());
            args.push(idx.to_string());
        } else {
            args.push("--no-playlist".to_string());
        }

        if job.save_metadata {
            args.push("--write-info-json".to_string());
        }
        if job.embed_thumbnail {
            args.push("--embed-thumbnail".to_string());
        }

        let _cookie_guard = self.append_auth_and_network_args(&mut args, settings)?;
        args.push("-o".to_string());
        args.push(target_path.to_string_lossy().to_string());
        args.push("--".to_string());
        args.push(canonical_url.to_string());

        let mut cmd = new_hidden_tokio_command(&bin);
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let pid = child.id();
        if let Some(p) = pid {
            self.process_registry.register(&job.id, p);
        }

        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stderr_collected = String::new();
        let mut current_stage = JobStatus::DownloadingVideo;

        loop {
            tokio::select! {
                Ok(Some(line)) = stdout_reader.next_line() => {
                    if let Some(payload) = parse_ytdlp_stdout_line(
                        &line,
                        &job.id,
                        item_index,
                        total_items,
                        &mut current_stage,
                    ) {
                        emitter.emit_if_ready(app, &payload, payload.status == JobStatus::Merging);
                    }
                }
                Ok(Some(err_line)) = stderr_reader.next_line() => {
                    stderr_collected.push_str(&err_line);
                    stderr_collected.push('\n');
                    if is_circuit_breaker_trigger(&err_line).is_some() {
                        if let Some(p) = pid {
                            self.process_registry.unregister(&job.id, p);
                        }
                        let _ = child.kill().await;
                        return Err(AppError::RateLimited { retry_after_secs: Some(180) });
                    }
                }
                status_res = child.wait() => {
                    if let Some(p) = pid {
                        self.process_registry.unregister(&job.id, p);
                    }
                    let status = status_res?;
                    if !status.success() {
                        return Err(AppError::from_engine_stderr(&stderr_collected));
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    async fn run_gallery_dl_slide(
        &self,
        canonical_url: &str,
        slide_index: usize,
        target_path: &Path,
        job_id: &str,
        settings: &AppSettings,
    ) -> AppResult<()> {
        let bin = self.binary_manager.gallery_dl_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing("gallery-dl.exe".to_string()));
        }

        let dest_dir = target_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_string_lossy()
            .to_string();
        let file_name = target_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut args = vec![
            "-D".to_string(),
            dest_dir,
            "-f".to_string(),
            file_name,
            "--range".to_string(),
            slide_index.to_string(),
            "--no-colors".to_string(),
        ];

        let _cookie_guard = self.append_auth_and_network_args(&mut args, settings)?;
        args.push("--".to_string());
        args.push(canonical_url.to_string());

        let mut cmd = new_hidden_tokio_command(&bin);
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn()?;
        let pid = child.id();
        if let Some(p) = pid {
            self.process_registry.register(job_id, p);
        }

        let output = child.wait_with_output().await?;
        if let Some(p) = pid {
            self.process_registry.unregister(job_id, p);
        }

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::from_engine_stderr(&stderr));
        }

        if !target_path.exists()
            || std::fs::metadata(target_path)
                .map(|m| m.len() == 0)
                .unwrap_or(true)
        {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.trim().is_empty() {
                return Err(AppError::from_engine_stderr(&stderr));
            }
            return Err(AppError::MediaDeletedOrUnavailable);
        }

        Ok(())
    }

    async fn download_direct_cdn_asset(&self, asset_url: &str, target_path: &Path) -> AppResult<()> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(60))
            .build()?;

        let resp = client
            .get(asset_url)
            .header("Referer", "https://www.facebook.com/")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::ProcessError(format!(
                "Direct CDN download failed with HTTP {}",
                resp.status()
            )));
        }

        let bytes = resp.bytes().await?;
        std::fs::write(target_path, &bytes)?;
        Ok(())
    }
}
