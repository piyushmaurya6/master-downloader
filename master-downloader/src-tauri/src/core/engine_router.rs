use crate::core::auth_vault::{AuthVault, EphemeralCookieGuard};
use crate::core::binary_manager::{BinaryManager, ProcessRegistry, CREATE_NO_WINDOW};
use crate::core::parser::{
    parse_gallery_dl_inspect_json, parse_instagram_graphql_dump_json, parse_ytdlp_inspect_json,
    parse_ytdlp_stdout_line, MediaInspectionResult, ProgressPayload, SlideMediaType,
};
use crate::core::url_normalizer::{
    normalize_media_url, resolve_output_path, sanitize_filename_component, MediaUrlType,
    NormalizedMediaUrl, Platform,
};
use crate::db::repository::{JobRecord, MediaFileRecord};
use crate::error::{AppError, AppResult};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use uuid::Uuid;

#[derive(Clone)]
pub struct EngineRouter {
    pub binaries: BinaryManager,
    pub vault: AuthVault,
    pub processes: ProcessRegistry,
    pub cache_dir: PathBuf,
}

impl EngineRouter {
    pub fn new(
        binaries: BinaryManager,
        vault: AuthVault,
        processes: ProcessRegistry,
        app_data_dir: &Path,
    ) -> AppResult<Self> {
        let cache_dir = app_data_dir.join("cache").join("thumbs");
        fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            binaries,
            vault,
            processes,
            cache_dir,
        })
    }

    /// Resolves Facebook `fb.watch/*` and `facebook.com/share/{v|r|p}/*` redirect tokens.
    pub async fn resolve_share_or_shortlink(&self, norm: &mut NormalizedMediaUrl) {
        if !norm.requires_redirect_resolution {
            return;
        }

        let Ok(client) = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(Duration::from_secs(12))
            .build()
        else {
            return;
        };

        if let Ok(resp) = client
            .get(&norm.canonical_url)
            .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
        {
            let final_url = resp.url().as_str().to_string();
            let candidate_target = if final_url.contains("/login") {
                resp.url()
                    .query_pairs()
                    .find(|(k, _)| k == "next")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or(final_url)
            } else {
                final_url
            };

            if !candidate_target.contains("/login") && candidate_target != norm.canonical_url {
                if let Ok(resolved) = normalize_media_url(&candidate_target) {
                    norm.canonical_url = resolved.canonical_url;
                    norm.url_type = resolved.url_type;
                    if resolved.shortcode.is_some() {
                        norm.shortcode = resolved.shortcode;
                    }
                    if resolved.username.is_some() {
                        norm.username = resolved.username;
                    }
                    norm.requires_redirect_resolution = false;
                }
            }
        }
    }

    /// Universal metadata inspection across YouTube, Instagram, Facebook, and generic URLs.
    pub async fn inspect_url(&self, raw_url: &str) -> AppResult<MediaInspectionResult> {
        let mut norm = normalize_media_url(raw_url)?;
        self.resolve_share_or_shortlink(&mut norm).await;

        let cookie_guard = self.vault.create_ephemeral_cookie_guard()?;
        let browser_source = self.vault.get_browser_delegation();

        match norm.platform {
            Platform::YouTube => {
                let mut res = self
                    .run_ytdlp_inspect(&norm, cookie_guard.as_ref(), browser_source.as_deref())
                    .await?;
                res.platform = Platform::YouTube;
                Ok(res)
            }
            Platform::Instagram => {
                // Stories, Highlights, and Profiles go to gallery-dl first
                let prefer_gallery_dl = matches!(
                    norm.url_type,
                    MediaUrlType::Stories | MediaUrlType::Highlight | MediaUrlType::Profile
                );

                if prefer_gallery_dl {
                    match self
                        .run_gallery_dl_inspect(
                            &norm,
                            cookie_guard.as_ref(),
                            browser_source.as_deref(),
                        )
                        .await
                    {
                        Ok(res) => return Ok(res),
                        Err(primary_err) => {
                            if let Ok(gql) = self
                                .run_ytdlp_graphql_inspect(
                                    &norm,
                                    cookie_guard.as_ref(),
                                    browser_source.as_deref(),
                                )
                                .await
                            {
                                return Ok(gql);
                            }
                            if let Ok(fallback) = self
                                .run_ytdlp_inspect(
                                    &norm,
                                    cookie_guard.as_ref(),
                                    browser_source.as_deref(),
                                )
                                .await
                            {
                                return Ok(fallback);
                            }
                            return Err(primary_err);
                        }
                    }
                }

                // For Instagram /p/ Posts (single photos, photo carousels, mixed carousels):
                // Try Polaris GraphQL dump (--write-pages --skip-download --playlist-items 1) FIRST
                // because it extracts 1080p images and carousels in ~2s without requiring login cookies,
                // whereas standard yt-dlp fails on image posts with 'NoneType' or 'No video formats found'.
                if norm.url_type == MediaUrlType::Post {
                    match self
                        .run_ytdlp_graphql_inspect(
                            &norm,
                            cookie_guard.as_ref(),
                            browser_source.as_deref(),
                        )
                        .await
                    {
                        Ok(gql_res) => return Ok(gql_res),
                        Err(gql_err) => {
                            if let Ok(yt_res) = self
                                .run_ytdlp_inspect(
                                    &norm,
                                    cookie_guard.as_ref(),
                                    browser_source.as_deref(),
                                )
                                .await
                            {
                                return Ok(yt_res);
                            }
                            if let Ok(gdl_res) = self
                                .run_gallery_dl_inspect(
                                    &norm,
                                    cookie_guard.as_ref(),
                                    browser_source.as_deref(),
                                )
                                .await
                            {
                                return Ok(gdl_res);
                            }
                            return Err(gql_err);
                        }
                    }
                }

                // Reels and IGTV try yt-dlp first (for full DASH format list), then Polaris GraphQL, then gallery-dl
                match self
                    .run_ytdlp_inspect(&norm, cookie_guard.as_ref(), browser_source.as_deref())
                    .await
                {
                    Ok(res) => Ok(res),
                    Err(yt_err) => {
                        if let Ok(gql_res) = self
                            .run_ytdlp_graphql_inspect(
                                &norm,
                                cookie_guard.as_ref(),
                                browser_source.as_deref(),
                            )
                            .await
                        {
                            return Ok(gql_res);
                        }

                        match self
                            .run_gallery_dl_inspect(
                                &norm,
                                cookie_guard.as_ref(),
                                browser_source.as_deref(),
                            )
                            .await
                        {
                            Ok(fallback) => Ok(fallback),
                            Err(gdl_err) => {
                                if matches!(
                                    gdl_err,
                                    AppError::AuthenticationRequired(_)
                                        | AppError::RateLimited(_)
                                        | AppError::PrivateAccount(_)
                                ) {
                                    Err(gdl_err)
                                } else {
                                    Err(yt_err)
                                }
                            }
                        }
                    }
                }
            }
            Platform::Facebook | Platform::Generic => {
                let prefer_gallery_dl = matches!(norm.url_type, MediaUrlType::Photo);
                if prefer_gallery_dl {
                    match self
                        .run_gallery_dl_inspect(
                            &norm,
                            cookie_guard.as_ref(),
                            browser_source.as_deref(),
                        )
                        .await
                    {
                        Ok(res) => return Ok(res),
                        Err(primary_err) => {
                            if let Ok(fallback) = self
                                .run_ytdlp_inspect(
                                    &norm,
                                    cookie_guard.as_ref(),
                                    browser_source.as_deref(),
                                )
                                .await
                            {
                                return Ok(fallback);
                            }
                            return Err(primary_err);
                        }
                    }
                }

                match self
                    .run_ytdlp_inspect(&norm, cookie_guard.as_ref(), browser_source.as_deref())
                    .await
                {
                    Ok(res) => Ok(res),
                    Err(yt_err) => {
                        match self
                            .run_gallery_dl_inspect(
                                &norm,
                                cookie_guard.as_ref(),
                                browser_source.as_deref(),
                            )
                            .await
                        {
                            Ok(fallback) => Ok(fallback),
                            Err(gdl_err) => {
                                if matches!(
                                    gdl_err,
                                    AppError::AuthenticationRequired(_)
                                        | AppError::RateLimited(_)
                                        | AppError::PrivateAccount(_)
                                ) {
                                    Err(gdl_err)
                                } else {
                                    Err(yt_err)
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn run_ytdlp_inspect(
        &self,
        norm: &NormalizedMediaUrl,
        cookie_guard: Option<&EphemeralCookieGuard>,
        browser_source: Option<&str>,
    ) -> AppResult<MediaInspectionResult> {
        let bin = self.binaries.yt_dlp_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing(
                "yt-dlp.exe is not installed in %LOCALAPPDATA%\\MasterDownloader\\bin".into(),
            ));
        }

        let mut cmd = Command::new(&bin);
        cmd.args([
            "--dump-single-json",
            "--no-warnings",
            "--no-check-certificates",
            "--socket-timeout",
            "20",
        ]);

        if norm.platform == Platform::YouTube {
            cmd.arg("--force-ipv4");
            let deno = self.binaries.deno_path();
            if deno.exists() {
                cmd.arg("--js-runtimes")
                    .arg(format!("deno:{}", deno.to_string_lossy()));
            }
            if norm.url_type == MediaUrlType::Playlist {
                cmd.args(["--flat-playlist", "--playlist-end", "50"]);
            } else {
                cmd.arg("--no-playlist");
            }
        } else {
            cmd.arg("--no-playlist");
        }

        Self::append_auth_args(&mut cmd, cookie_guard, browser_source, true);
        cmd.arg("--").arg(&norm.canonical_url);

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = tokio::time::timeout(Duration::from_secs(45), cmd.output())
            .await
            .map_err(|_| AppError::Network("yt-dlp metadata inspection timed out.".into()))??;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !stdout.trim().is_empty() && stdout.trim() != "null" {
            if let Ok(parsed) = parse_ytdlp_inspect_json(
                &stdout,
                norm.shortcode.as_deref(),
                Some(norm.platform.clone()),
            ) {
                return Ok(parsed);
            }
        }

        if !output.status.success() {
            return Err(AppError::classify_engine_stderr(&stderr));
        }

        parse_ytdlp_inspect_json(&stdout, norm.shortcode.as_deref(), Some(norm.platform.clone()))
    }

    /// Runs `yt-dlp --write-pages --skip-download --playlist-items 1` inside an isolated temp directory to capture
    /// Instagram's Polaris GraphQL dump for public single-photo and 1080p carousel posts.
    async fn run_ytdlp_graphql_inspect(
        &self,
        norm: &NormalizedMediaUrl,
        cookie_guard: Option<&EphemeralCookieGuard>,
        browser_source: Option<&str>,
    ) -> AppResult<MediaInspectionResult> {
        let bin = self.binaries.yt_dlp_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing("yt-dlp.exe missing".into()));
        }

        let scratch_dir = self
            .cache_dir
            .join(format!("ig_gql_{}", Uuid::new_v4()));
        fs::create_dir_all(&scratch_dir)?;

        let mut cmd = Command::new(&bin);
        cmd.current_dir(&scratch_dir);
        cmd.args([
            "--write-pages",
            "--skip-download",
            "--no-warnings",
            "--socket-timeout",
            "20",
            "--playlist-items",
            "1",
        ]);
        Self::append_auth_args(&mut cmd, cookie_guard, browser_source, true);
        cmd.arg("--").arg(&norm.canonical_url);

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let _ = tokio::time::timeout(Duration::from_secs(30), cmd.output()).await;

        let mut parsed_result: Option<MediaInspectionResult> = None;
        if let Ok(entries) = fs::read_dir(&scratch_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Ok(content) = fs::read_to_string(&p) {
                        if content.contains("xig_polaris_media")
                            || content.contains("xdt_shortcode_media")
                            || content.contains("shortcode_media")
                        {
                            if let Ok(res) = parse_instagram_graphql_dump_json(
                                &content,
                                norm.shortcode.as_deref(),
                            ) {
                                parsed_result = Some(res);
                                break;
                            }
                        }
                    }
                }
            }
        }

        let _ = fs::remove_dir_all(&scratch_dir);
        parsed_result.ok_or_else(|| {
            AppError::MediaUnavailable("Instagram GraphQL fallback returned no media.".into())
        })
    }

    async fn run_gallery_dl_inspect(
        &self,
        norm: &NormalizedMediaUrl,
        cookie_guard: Option<&EphemeralCookieGuard>,
        browser_source: Option<&str>,
    ) -> AppResult<MediaInspectionResult> {
        let bin = self.binaries.gallery_dl_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing(
                "gallery-dl.exe is not installed in %LOCALAPPDATA%\\MasterDownloader\\bin".into(),
            ));
        }

        let mut cmd = Command::new(&bin);
        cmd.args(["--dump-json", "--no-download"]);
        Self::append_auth_args(&mut cmd, cookie_guard, browser_source, false);
        cmd.arg("--").arg(&norm.canonical_url);

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let output = tokio::time::timeout(Duration::from_secs(40), cmd.output())
            .await
            .map_err(|_| AppError::Network("gallery-dl metadata inspection timed out.".into()))??;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && stdout.trim().is_empty() {
            return Err(AppError::classify_engine_stderr(&stderr));
        }

        parse_gallery_dl_inspect_json(
            &stdout,
            &norm.canonical_url,
            norm.shortcode.as_deref(),
            norm.platform.clone(),
        )
    }

    /// Executes a queued download job across YouTube, Instagram, Facebook, or Generic URLs.
    pub async fn execute_job(
        &self,
        job: &JobRecord,
        organize_by_platform: bool,
        organize_by_uploader: bool,
        app: Option<&AppHandle>,
    ) -> AppResult<Vec<MediaFileRecord>> {
        let mut norm = normalize_media_url(&job.source_url)?;
        self.resolve_share_or_shortlink(&mut norm).await;

        let cookie_guard = self.vault.create_ephemeral_cookie_guard()?;
        let browser_source = self.vault.get_browser_delegation();

        let base_dir = PathBuf::from(&job.output_dir);
        fs::create_dir_all(&base_dir)?;

        let date_str = chrono::Utc::now().format("%Y-%m-%d").to_string();

        // Instagram Stories / Highlights / Profiles or Facebook Photos -> gallery-dl
        let is_gallery_only = matches!(
            norm.url_type,
            MediaUrlType::Stories
                | MediaUrlType::Highlight
                | MediaUrlType::Profile
                | MediaUrlType::Photo
        ) && !job.quality_preset.starts_with("audio_");

        if is_gallery_only {
            return self
                .execute_gallery_dl_download(
                    job,
                    &norm,
                    &base_dir,
                    &date_str,
                    organize_by_platform,
                    organize_by_uploader,
                    cookie_guard.as_ref(),
                    browser_source.as_deref(),
                    app,
                )
                .await;
        }

        // For Instagram Posts/Carousels, inspect first so we can route mixed carousels or direct GraphQL images cleanly
        if norm.platform == Platform::Instagram && norm.url_type == MediaUrlType::Post {
            if let Ok(inspected) = self.inspect_url(&norm.canonical_url).await {
                if !job.quality_preset.starts_with("audio_")
                    && inspected.slides.iter().any(|s| s.direct_url.is_some())
                {
                    if let Ok(files) = self
                        .execute_direct_slides_download(
                            job,
                            &inspected,
                            &base_dir,
                            &date_str,
                            organize_by_platform,
                            organize_by_uploader,
                            app,
                        )
                        .await
                    {
                        if !files.is_empty() {
                            return Ok(files);
                        }
                    }
                }
            }
        }

        match self
            .execute_ytdlp_download(
                job,
                &norm,
                &base_dir,
                &date_str,
                organize_by_platform,
                organize_by_uploader,
                cookie_guard.as_ref(),
                browser_source.as_deref(),
                app,
            )
            .await
        {
            Ok(files) if !files.is_empty() => Ok(files),
            Err(e) if self.processes.is_job_aborted(&job.id) => Err(e),
            Ok(_) | Err(_) if !job.quality_preset.starts_with("audio_") && norm.platform != Platform::YouTube => {
                if self.processes.is_job_aborted(&job.id) {
                    return Err(AppError::Internal("Job paused or cancelled.".into()));
                }
                if let Ok(inspected) = self.inspect_url(&norm.canonical_url).await {
                    if inspected.slides.iter().any(|s| s.direct_url.is_some()) {
                        if let Ok(files) = self
                            .execute_direct_slides_download(
                                job,
                                &inspected,
                                &base_dir,
                                &date_str,
                                organize_by_platform,
                                organize_by_uploader,
                                app,
                            )
                            .await
                        {
                            if !files.is_empty() {
                                return Ok(files);
                            }
                        }
                    }
                }
                self.execute_gallery_dl_download(
                    job,
                    &norm,
                    &base_dir,
                    &date_str,
                    organize_by_platform,
                    organize_by_uploader,
                    cookie_guard.as_ref(),
                    browser_source.as_deref(),
                    app,
                )
                .await
            }
            Err(e) => Err(e),
            Ok(files) => Ok(files),
        }
    }

    async fn execute_direct_slides_download(
        &self,
        job: &JobRecord,
        inspected: &MediaInspectionResult,
        base_dir: &Path,
        date_str: &str,
        organize_by_platform: bool,
        organize_by_uploader: bool,
        app: Option<&AppHandle>,
    ) -> AppResult<Vec<MediaFileRecord>> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(60))
            .build()?;

        let referer = match inspected.platform {
            Platform::Instagram => "https://www.instagram.com/",
            Platform::Facebook => "https://www.facebook.com/",
            Platform::YouTube => "https://www.youtube.com/",
            Platform::Generic => "https://www.google.com/",
        };

        let selected: Vec<usize> = if job.selected_indices.is_empty() {
            (1..=inspected.slides.len()).collect()
        } else {
            job.selected_indices.clone()
        };

        let mut records = Vec::new();
        let total_slides = selected.len().max(1);

        for (pos, slide_idx) in selected.iter().enumerate() {
            if self.processes.is_job_aborted(&job.id) {
                return Err(AppError::Internal("Job paused or cancelled.".into()));
            }
            let Some(slide) = inspected.slides.iter().find(|s| s.index == *slide_idx) else {
                continue;
            };
            let Some(ref direct_url) = slide.direct_url else {
                continue;
            };

            let ext = if slide.media_type == SlideMediaType::Video {
                "mp4"
            } else {
                "jpg"
            };

            let target_path = resolve_output_path(
                base_dir,
                inspected.platform.as_str(),
                Some(&inspected.author_username),
                inspected.shortcode.as_deref(),
                date_str,
                *slide_idx,
                ext,
                organize_by_platform,
                organize_by_uploader,
            );

            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }

            let resp = client
                .get(direct_url)
                .header("Referer", referer)
                .send()
                .await?;

            if !resp.status().is_success() {
                return Err(AppError::Network(format!(
                    "Direct CDN download returned HTTP {}",
                    resp.status()
                )));
            }

            let bytes = resp.bytes().await?;
            fs::write(&target_path, &bytes)?;

            let pct = ((pos + 1) as f64 / total_slides as f64) * 100.0;
            if let Some(handle) = app {
                let _ = handle.emit(
                    "download-progress",
                    ProgressPayload {
                        job_id: job.id.clone(),
                        slide_index: *slide_idx,
                        status: "downloading".into(),
                        downloaded_bytes: bytes.len() as u64,
                        total_bytes: Some(bytes.len() as u64),
                        speed_bps: None,
                        eta_seconds: Some(0),
                        percentage: pct,
                    },
                );
            }

            records.push(MediaFileRecord {
                id: Uuid::new_v4().to_string(),
                job_id: job.id.clone(),
                slide_index: *slide_idx,
                media_type: if slide.media_type == SlideMediaType::Video {
                    "video".into()
                } else {
                    "image".into()
                },
                file_path: target_path.to_string_lossy().to_string(),
                file_size_bytes: Some(bytes.len() as u64),
                width: slide.width,
                height: slide.height,
                duration_seconds: slide.duration_seconds,
            });
        }

        if let Some(first) = records.first() {
            if let Some(parent) = Path::new(&first.file_path).parent() {
                self.write_sidecars_if_requested(job, parent, date_str);
            }
        }

        Ok(records)
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_ytdlp_download(
        &self,
        job: &JobRecord,
        norm: &NormalizedMediaUrl,
        base_dir: &Path,
        date_str: &str,
        organize_by_platform: bool,
        organize_by_uploader: bool,
        cookie_guard: Option<&EphemeralCookieGuard>,
        browser_source: Option<&str>,
        app: Option<&AppHandle>,
    ) -> AppResult<Vec<MediaFileRecord>> {
        let bin = self.binaries.yt_dlp_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing("yt-dlp.exe is not installed.".into()));
        }

        let target_sample = resolve_output_path(
            base_dir,
            norm.platform.as_str(),
            job.author_username.as_deref().or(norm.username.as_deref()),
            job.shortcode.as_deref().or(norm.shortcode.as_deref()),
            date_str,
            1,
            "mp4",
            organize_by_platform,
            organize_by_uploader,
        );

        let mut target_folder = target_sample
            .parent()
            .unwrap_or(base_dir)
            .to_path_buf();

        let is_playlist_download = job.options.playlist_mode.as_deref() == Some("playlist")
            || norm.url_type == MediaUrlType::Playlist;

        if is_playlist_download && norm.platform == Platform::YouTube {
            if let Some(ref title) = job.caption {
                let safe_playlist = sanitize_filename_component(title, 48);
                target_folder = target_folder.join(safe_playlist);
            }
        }

        fs::create_dir_all(&target_folder)?;

        let safe_user = sanitize_filename_component(
            job.author_username
                .as_deref()
                .or(norm.username.as_deref())
                .unwrap_or("creator"),
            36,
        );
        let safe_code = sanitize_filename_component(
            job.shortcode
                .as_deref()
                .or(norm.shortcode.as_deref())
                .unwrap_or("media"),
            28,
        );

        let unique_stem = target_sample
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("media_01")
            .to_string();

        let output_template = if is_playlist_download && norm.platform == Platform::YouTube {
            target_folder
                .join("%(playlist_index)02d_%(title).60s_%(id)s.%(ext)s")
                .to_string_lossy()
                .to_string()
        } else if job.selected_indices.len() > 1 {
            target_folder
                .join(format!(
                    "{}_{}_{}_%(autonumber)02d.%(ext)s",
                    safe_user, date_str, safe_code
                ))
                .to_string_lossy()
                .to_string()
        } else {
            target_folder
                .join(format!("{}.%(ext)s", unique_stem))
                .to_string_lossy()
                .to_string()
        };

        let mut cmd = Command::new(&bin);
        cmd.args([
            "--newline",
            "--no-warnings",
            "--no-check-certificates",
            "--continue",
            "--progress-template",
            "download:DL_PROGRESS|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.eta)s|%(progress.speed)s|%(progress._percent_str)s",
            "--print",
            "after_move:DL_FILEPATH|%(filepath)s",
            "-o",
            &output_template,
        ]);

        // Pass Deno runtime & IPv4 preference for YouTube signature/challenge solving
        if norm.platform == Platform::YouTube {
            cmd.arg("--force-ipv4");
            let deno = self.binaries.deno_path();
            if deno.exists() {
                cmd.arg("--js-runtimes")
                    .arg(format!("deno:{}", deno.to_string_lossy()));
            }
            if is_playlist_download {
                cmd.arg("--yes-playlist");
            } else {
                cmd.arg("--no-playlist");
            }
        } else if job.selected_indices.len() <= 1 {
            cmd.arg("--no-playlist");
        }

        let ffmpeg_dir = self.binaries.bin_dir.to_string_lossy().to_string();
        cmd.args(["--ffmpeg-location", &ffmpeg_dir]);

        // Speed limit
        if let Some(kbps) = job.options.speed_limit_kbps {
            if kbps > 0 {
                cmd.args(["--limit-rate", &format!("{}K", kbps)]);
            }
        }

        // Audio vs Video format matrix
        let preset = job.quality_preset.as_str();
        let is_audio_only = preset.starts_with("audio_");

        if is_audio_only {
            let audio_fmt = match preset {
                "audio_m4a" => "m4a",
                "audio_flac" => "flac",
                "audio_wav" => "wav",
                "audio_opus" => "opus",
                _ => job.options.audio_format.as_deref().unwrap_or("mp3"),
            };
            let bitrate = job.options.audio_bitrate.as_deref().unwrap_or("320");
            let quality_arg = match bitrate {
                "320" | "best" => "0",
                "256" => "2",
                "192" => "4",
                "128" => "6",
                _ => "0",
            };
            cmd.args([
                "-x",
                "--audio-format",
                audio_fmt,
                "--audio-quality",
                quality_arg,
            ]);

            if job.options.audio_normalization {
                cmd.args([
                    "--postprocessor-args",
                    "ExtractAudio+ffmpeg:-af loudnorm=I=-16:TP=-1.5:LRA=11",
                ]);
            }
        } else {
            let container = job
                .options
                .container
                .as_deref()
                .filter(|c| *c != "auto" && !c.is_empty())
                .unwrap_or("mp4");

            let height_target: Option<u32> = match preset {
                "4k" | "2160p" => Some(2160),
                "1440p" => Some(1440),
                "1080p60" | "1080p" => Some(1080),
                "720p" => Some(720),
                "480p" => Some(480),
                "360p" => Some(360),
                _ => None,
            };

            let fps_filter = if preset == "1080p60" || job.options.preferred_fps == Some(60) {
                "[fps>=50]"
            } else {
                ""
            };

            let codec_filter = match job.options.preferred_codec.as_deref() {
                Some("h264") => "[vcodec^=avc1]",
                Some("h265") => "[vcodec^=hev1]",
                Some("vp9") => "[vcodec^=vp9]",
                Some("av1") => "[vcodec^=av01]",
                _ => "",
            };

            let format_expr = match height_target {
                Some(h) if job.options.exact_resolution => format!(
                    "bestvideo[height={}]{}{}+bestaudio/best[height={}]/best",
                    h, fps_filter, codec_filter, h
                ),
                Some(h) => format!(
                    "bestvideo[height<={}][ext=mp4]{}{}+bestaudio[ext=m4a]/bestvideo[height<={}]{}{}+bestaudio/best[height<={}]/hd/sd/best",
                    h, fps_filter, codec_filter, h, fps_filter, codec_filter, h
                ),
                None => format!(
                    "bestvideo[ext=mp4]{}{}+bestaudio[ext=m4a]/bestvideo{}{}+bestaudio/best[ext=mp4]/hd/sd/best",
                    fps_filter, codec_filter, fps_filter, codec_filter
                ),
            };

            cmd.args([
                "-f",
                &format_expr,
                "--merge-output-format",
                container,
            ]);
        }

        if job.options.write_subtitles {
            let langs = job
                .options
                .subtitle_langs
                .as_deref()
                .unwrap_or("en,.*-orig");
            cmd.args(["--write-subs", "--write-auto-subs", "--sub-langs", langs]);
        }

        if job.options.embed_chapters {
            cmd.arg("--embed-chapters");
        }

        if !job.selected_indices.is_empty() && !is_playlist_download && job.selected_indices.len() > 1 {
            let items_csv = job
                .selected_indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            cmd.args(["--playlist-items", &items_csv]);
        }

        if job.embed_thumbnail {
            cmd.arg("--embed-metadata");
        }

        Self::append_auth_args(&mut cmd, cookie_guard, browser_source, true);
        cmd.arg("--").arg(&norm.canonical_url);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let before_files = list_files_in_dir(&target_folder);

        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);
        if pid > 0 {
            self.processes.register_job_pid(&job.id, pid);
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Internal("Failed to attach yt-dlp stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Internal("Failed to attach yt-dlp stderr".into()))?;

        let job_id_clone = job.id.clone();
        let app_clone = app.cloned();
        let stdout_task = tokio::spawn(async move {
            let mut printed_paths: Vec<PathBuf> = Vec::new();
            let mut reader = BufReader::new(stdout).lines();
            let mut last_emit: Option<Instant> = None;
            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim();
                if let Some(fp) = trimmed.strip_prefix("DL_FILEPATH|") {
                    let p = PathBuf::from(fp.trim());
                    if p.exists() && !printed_paths.contains(&p) {
                        printed_paths.push(p);
                    }
                    continue;
                }
                if let Some(progress) = parse_ytdlp_stdout_line(&job_id_clone, 1, &line) {
                    let is_phase_change = progress.status != "downloading" || progress.percentage >= 99.5;
                    let should_emit = is_phase_change
                        || last_emit
                            .map(|t| t.elapsed() >= Duration::from_millis(180))
                            .unwrap_or(true);
                    if should_emit {
                        last_emit = Some(Instant::now());
                        if let Some(ref handle) = app_clone {
                            let _ = handle.emit("download-progress", progress);
                        }
                    }
                }
            }
            printed_paths
        });

        let stderr_task = tokio::spawn(async move {
            let mut buf = String::new();
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                buf.push_str(&line);
                buf.push('\n');
            }
            buf
        });

        let status = child.wait().await?;
        if pid > 0 {
            self.processes.unregister_job_pid(&job.id, pid);
        }
        let printed_paths = stdout_task.await.unwrap_or_default();
        let stderr_text = stderr_task.await.unwrap_or_default();

        if !status.success() && printed_paths.is_empty() {
            return Err(AppError::classify_engine_stderr(&stderr_text));
        }

        self.write_sidecars_if_requested(job, &target_folder, date_str);

        if !printed_paths.is_empty() {
            return Ok(collect_new_media_records(&job.id, &[], &printed_paths));
        }

        let after_files = list_files_in_dir(&target_folder);
        Ok(collect_new_media_records(&job.id, &before_files, &after_files))
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_gallery_dl_download(
        &self,
        job: &JobRecord,
        norm: &NormalizedMediaUrl,
        base_dir: &Path,
        date_str: &str,
        organize_by_platform: bool,
        organize_by_uploader: bool,
        cookie_guard: Option<&EphemeralCookieGuard>,
        browser_source: Option<&str>,
        app: Option<&AppHandle>,
    ) -> AppResult<Vec<MediaFileRecord>> {
        let bin = self.binaries.gallery_dl_path();
        if !bin.exists() {
            return Err(AppError::BinaryMissing("gallery-dl.exe is not installed.".into()));
        }

        let target_sample = resolve_output_path(
            base_dir,
            norm.platform.as_str(),
            job.author_username.as_deref().or(norm.username.as_deref()),
            job.shortcode.as_deref().or(norm.shortcode.as_deref()),
            date_str,
            1,
            "jpg",
            organize_by_platform,
            organize_by_uploader,
        );
        let target_folder = target_sample
            .parent()
            .unwrap_or(base_dir)
            .to_path_buf();
        fs::create_dir_all(&target_folder)?;

        let mut cmd = Command::new(&bin);
        cmd.args([
            "--directory",
            &target_folder.to_string_lossy(),
            "--filename",
            "{username}_{post_shortcode:?/_/}{num:>02}.{extension}",
        ]);

        if !job.selected_indices.is_empty() {
            let range_expr = job
                .selected_indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            cmd.args(["--range", &range_expr]);
        }

        Self::append_auth_args(&mut cmd, cookie_guard, browser_source, false);
        cmd.arg("--").arg(&norm.canonical_url);

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let before_files = list_files_in_dir(&target_folder);

        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);
        if pid > 0 {
            self.processes.register_job_pid(&job.id, pid);
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Internal("Failed to attach gallery-dl stdout".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Internal("Failed to attach gallery-dl stderr".into()))?;

        let job_id_clone = job.id.clone();
        let app_clone = app.cloned();
        let total_expected = job.selected_indices.len().max(1);

        let stdout_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            let mut completed_slides = 0usize;
            while let Ok(Some(line)) = reader.next_line().await {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    completed_slides += 1;
                    let pct = ((completed_slides as f64 / total_expected as f64) * 100.0)
                        .clamp(5.0, 99.0);
                    if let Some(ref handle) = app_clone {
                        let _ = handle.emit(
                            "download-progress",
                            ProgressPayload {
                                job_id: job_id_clone.clone(),
                                slide_index: completed_slides,
                                status: "downloading".into(),
                                downloaded_bytes: 0,
                                total_bytes: None,
                                speed_bps: None,
                                eta_seconds: None,
                                percentage: pct,
                            },
                        );
                    }
                }
            }
        });

        let stderr_task = tokio::spawn(async move {
            let mut buf = String::new();
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                buf.push_str(&line);
                buf.push('\n');
            }
            buf
        });

        let status = child.wait().await?;
        if pid > 0 {
            self.processes.unregister_job_pid(&job.id, pid);
        }
        let _ = stdout_task.await;
        let stderr_text = stderr_task.await.unwrap_or_default();

        if !status.success() {
            return Err(AppError::classify_engine_stderr(&stderr_text));
        }

        self.write_sidecars_if_requested(job, &target_folder, date_str);

        let after_files = list_files_in_dir(&target_folder);
        Ok(collect_new_media_records(&job.id, &before_files, &after_files))
    }

    fn append_auth_args(
        cmd: &mut Command,
        cookie_guard: Option<&EphemeralCookieGuard>,
        browser_source: Option<&str>,
        _is_ytdlp: bool,
    ) {
        if let Some(guard) = cookie_guard {
            cmd.arg("--cookies").arg(&guard.path);
        } else if let Some(browser) = browser_source {
            if !browser.is_empty() && browser != "none" {
                cmd.args(["--cookies-from-browser", browser]);
            }
        }
    }

    fn write_sidecars_if_requested(&self, job: &JobRecord, target_folder: &Path, date_str: &str) {
        let safe_code = sanitize_filename_component(
            job.shortcode.as_deref().unwrap_or("post"),
            32,
        );

        if job.save_caption {
            if let Some(ref cap) = job.caption {
                let txt_path = target_folder.join(format!("{}_{}_caption.txt", safe_code, date_str));
                let _ = fs::write(txt_path, cap);
            }
        }

        if job.save_metadata {
            let json_path = target_folder.join(format!("{}_{}_metadata.json", safe_code, date_str));
            if let Ok(serialized) = serde_json::to_string_pretty(job) {
                let _ = fs::write(json_path, serialized);
            }
        }
    }

    /// Proxies and caches remote thumbnails (YouTube, Instagram, Facebook CDNs) to local disk.
    pub async fn proxy_thumbnail(&self, remote_url: &str) -> AppResult<String> {
        if remote_url.trim().is_empty() {
            return Err(AppError::InvalidUrl("Empty thumbnail URL".into()));
        }

        let mut hasher = Sha256::new();
        hasher.update(remote_url.as_bytes());
        let hash = hex::encode(hasher.finalize());
        let cached_path = self.cache_dir.join(format!("{}.jpg", &hash[..24]));

        if cached_path.exists() {
            if let Ok(bytes) = fs::read(&cached_path) {
                if !bytes.is_empty() {
                    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
                    return Ok(format!("data:image/jpeg;base64,{}", BASE64.encode(&bytes)));
                }
            }
        }

        let lower = remote_url.to_lowercase();
        let referer = if lower.contains("instagram") || lower.contains("cdninstagram") {
            "https://www.instagram.com/"
        } else if lower.contains("ytimg") || lower.contains("ggpht") || lower.contains("youtube") {
            "https://www.youtube.com/"
        } else {
            "https://www.facebook.com/"
        };

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(15))
            .build()?;

        let resp = client
            .get(remote_url)
            .header("Referer", referer)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(AppError::Network(format!(
                "Thumbnail HTTP {}",
                resp.status()
            )));
        }

        let bytes = resp.bytes().await?;
        let _ = fs::write(&cached_path, &bytes);
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        Ok(format!("data:image/jpeg;base64,{}", BASE64.encode(&bytes)))
    }
}

fn list_files_in_dir(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok().map(|entry| entry.path()))
        .filter(|p| p.is_file())
        .collect()
}

fn collect_new_media_records(
    job_id: &str,
    before: &[PathBuf],
    after: &[PathBuf],
) -> Vec<MediaFileRecord> {
    let mut records = Vec::new();
    let mut new_paths: Vec<PathBuf> = after
        .iter()
        .filter(|p| !before.contains(p))
        .filter(|p| {
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            matches!(
                ext.as_str(),
                "mp4" | "mkv" | "webm" | "mov" | "jpg" | "jpeg" | "png" | "webp" | "mp3" | "m4a" | "flac" | "wav" | "opus"
            )
        })
        .cloned()
        .collect();

    new_paths.sort();

    for (idx, path) in new_paths.into_iter().enumerate() {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("mp4")
            .to_lowercase();
        let media_type = match ext.as_str() {
            "mp4" | "mkv" | "webm" | "mov" => "video",
            "mp3" | "m4a" | "flac" | "wav" | "opus" => "audio",
            _ => "image",
        };
        let size = fs::metadata(&path).ok().map(|m| m.len());

        records.push(MediaFileRecord {
            id: Uuid::new_v4().to_string(),
            job_id: job_id.to_string(),
            slide_index: idx + 1,
            media_type: media_type.to_string(),
            file_path: path.to_string_lossy().to_string(),
            file_size_bytes: size,
            width: None,
            height: None,
            duration_seconds: None,
        });
    }

    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::repository::AdvancedJobOptions;

    #[tokio::test]
    async fn test_live_multi_platform_inspect_and_download() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bundled_bin = manifest_dir.join("resources").join("bin");
        if !bundled_bin.join("yt-dlp.exe").exists() {
            panic!("yt-dlp.exe not found in {:?}", bundled_bin);
        }

        let temp_root = std::env::temp_dir().join(format!("master_dl_live_test_{}", Uuid::new_v4()));
        let out_dir = temp_root.join("downloads");
        fs::create_dir_all(&out_dir).unwrap();

        let binaries = BinaryManager::new(&temp_root, Some(bundled_bin)).unwrap();
        let vault = AuthVault::new(&temp_root).unwrap();
        let processes = ProcessRegistry::new();
        let router = EngineRouter::new(binaries, vault, processes, &temp_root).unwrap();

        let test_cases = vec![
            (
                "Instagram 7-Slide Carousel",
                "https://www.instagram.com/p/DfXZgm6kd0g/",
                "instagram",
                "post",
                "best",
            ),
            (
                "Instagram Reel Video",
                "https://www.instagram.com/reel/DfHoldgoC1i/",
                "instagram",
                "reel",
                "best",
            ),
            (
                "Facebook Watch Video",
                "https://www.facebook.com/watch/?v=10153231379946729",
                "facebook",
                "watch",
                "720p",
            ),
            (
                "YouTube Video with Radio Mix List Param",
                "https://www.youtube.com/watch?v=OxXKDGO-MYQ&list=RDOxXKDGO-MYQ",
                "youtube",
                "video",
                "480p",
            ),
            (
                "YouTube Short Video (Me at the zoo)",
                "https://www.youtube.com/watch?v=jNQXAC9IVRw",
                "youtube",
                "video",
                "best",
            ),
        ];

        for (label, raw_url, expected_platform, expected_type, preset) in test_cases {
            println!("=== Testing [{}]: {} ===", label, raw_url);
            let norm = normalize_media_url(raw_url).expect("URL should normalize");
            assert_eq!(norm.platform.as_str(), expected_platform);
            assert_eq!(norm.url_type.as_str(), expected_type);

            let meta = router
                .inspect_url(raw_url)
                .await
                .unwrap_or_else(|e| panic!("inspect_url failed for {}: {:?}", label, e));
            println!(
                "  -> Inspected {}: author=@{}, slides={}",
                label,
                meta.author_username,
                meta.slides.len()
            );
            assert!(!meta.slides.is_empty(), "Slides should not be empty for {}", label);

            let job = JobRecord {
                id: Uuid::new_v4().to_string(),
                source_url: norm.canonical_url.clone(),
                platform: norm.platform.as_str().to_string(),
                shortcode: norm.shortcode.clone().or_else(|| meta.shortcode.clone()),
                url_type: norm.url_type.as_str().to_string(),
                author_username: Some(meta.author_username.clone()),
                caption: meta.caption.clone(),
                thumbnail_url: meta.slides.first().and_then(|s| s.thumbnail_url.clone()),
                status: "running".into(),
                selected_indices: Vec::new(),
                quality_preset: preset.into(),
                output_dir: out_dir.to_string_lossy().to_string(),
                error_message: None,
                save_caption: false,
                save_metadata: false,
                embed_thumbnail: false,
                naming_template: None,
                options: AdvancedJobOptions {
                    container: Some("mp4".into()),
                    playlist_mode: Some("single".into()),
                    ..Default::default()
                },
                progress_percent: 0.0,
                downloaded_bytes: 0,
                total_bytes: None,
                created_at: 0,
                completed_at: None,
                media_files: Vec::new(),
            };

            let files = router
                .execute_job(&job, true, true, None)
                .await
                .unwrap_or_else(|e| panic!("execute_job failed for {}: {:?}", label, e));

            println!("  -> Downloaded {} file(s) for {}", files.len(), label);
            assert!(!files.is_empty(), "Downloaded files should not be empty for {}", label);
            if label.contains("7-Slide Carousel") {
                assert_eq!(files.len(), 7, "Expected all 7 carousel slides to be downloaded");
            }
            for f in &files {
                let p = PathBuf::from(&f.file_path);
                assert!(p.exists(), "Downloaded file must exist on disk: {:?}", p);
                let sz = f.file_size_bytes.unwrap_or(0);
                assert!(sz > 1000, "Downloaded file must be > 1KB, got {} bytes", sz);
                println!("     * {} ({} bytes)", f.file_path, sz);
            }
        }

        let _ = fs::remove_dir_all(&temp_root);
    }
}
