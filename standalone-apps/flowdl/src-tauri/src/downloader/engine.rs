use crate::dependencies::ytdlp_service::{
    detect_ytdlp, fetch_latest_ytdlp_release, verify_ytdlp_binary,
};
use crate::downloader::download_manager::build_ytdlp_arguments;
use crate::models::{
    AppSettings, DownloadRequest, EngineUpdateInfo, MediaMetadata, ToolStatus, VideoFormatOption,
};
use crate::services::metadata_service::analyze_url_metadata;
use crate::utils::DiagnosticLogger;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Future-proof DownloadEngine adapter interface (Section 3).
/// Decouples higher-level application services from raw command-line arguments and engine specifics.
pub trait DownloadEngine: Send + Sync {
    fn engine_name(&self) -> &'static str;

    fn analyze_url<'a>(
        &'a self,
        url: &'a str,
        settings: &'a AppSettings,
        logger: &'a DiagnosticLogger,
    ) -> BoxFuture<'a, Result<MediaMetadata, String>>;

    fn get_available_formats<'a>(
        &'a self,
        url: &'a str,
        settings: &'a AppSettings,
        logger: &'a DiagnosticLogger,
    ) -> BoxFuture<'a, Result<Vec<VideoFormatOption>, String>>;

    fn build_download_invocation(
        &self,
        req: &DownloadRequest,
        settings: &AppSettings,
        ffmpeg_dir: Option<&Path>,
    ) -> Result<(Vec<String>, String, String), String>;

    fn get_engine_version<'a>(
        &'a self,
        settings: &'a AppSettings,
    ) -> BoxFuture<'a, Option<String>>;

    fn check_engine_health<'a>(&'a self, settings: &'a AppSettings) -> BoxFuture<'a, ToolStatus>;

    fn check_for_updates<'a>(
        &'a self,
        settings: &'a AppSettings,
    ) -> BoxFuture<'a, Result<EngineUpdateInfo, String>>;
}

/// Primary download engine powered by a bundled/managed `yt-dlp` binary and `FFmpeg`.
#[derive(Clone, Default)]
pub struct YtDlpEngine;

impl YtDlpEngine {
    pub fn new() -> Self {
        Self
    }

    pub async fn resolve_binary_path(&self, settings: &AppSettings) -> Result<PathBuf, String> {
        let status = detect_ytdlp(
            settings.ytdlp_path.as_deref(),
            settings.prefer_managed_binaries,
        )
        .await;
        status
            .path
            .map(PathBuf::from)
            .ok_or_else(|| "DEPENDENCY_MISSING||yt-dlp is not installed. Run repair in Settings.".to_string())
    }
}

impl DownloadEngine for YtDlpEngine {
    fn engine_name(&self) -> &'static str {
        "yt-dlp"
    }

    fn analyze_url<'a>(
        &'a self,
        url: &'a str,
        settings: &'a AppSettings,
        logger: &'a DiagnosticLogger,
    ) -> BoxFuture<'a, Result<MediaMetadata, String>> {
        Box::pin(async move {
            let bin = self.resolve_binary_path(settings).await?;
            analyze_url_metadata(&bin, url, logger).await
        })
    }

    fn get_available_formats<'a>(
        &'a self,
        url: &'a str,
        settings: &'a AppSettings,
        logger: &'a DiagnosticLogger,
    ) -> BoxFuture<'a, Result<Vec<VideoFormatOption>, String>> {
        Box::pin(async move {
            let meta = self.analyze_url(url, settings, logger).await?;
            Ok(meta.formats)
        })
    }

    fn build_download_invocation(
        &self,
        req: &DownloadRequest,
        settings: &AppSettings,
        ffmpeg_dir: Option<&Path>,
    ) -> Result<(Vec<String>, String, String), String> {
        build_ytdlp_arguments(req, settings, ffmpeg_dir)
    }

    fn get_engine_version<'a>(
        &'a self,
        settings: &'a AppSettings,
    ) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            let bin = self.resolve_binary_path(settings).await.ok()?;
            verify_ytdlp_binary(&bin).await.ok()
        })
    }

    fn check_engine_health<'a>(&'a self, settings: &'a AppSettings) -> BoxFuture<'a, ToolStatus> {
        Box::pin(async move {
            detect_ytdlp(
                settings.ytdlp_path.as_deref(),
                settings.prefer_managed_binaries,
            )
            .await
        })
    }

    fn check_for_updates<'a>(
        &'a self,
        settings: &'a AppSettings,
    ) -> BoxFuture<'a, Result<EngineUpdateInfo, String>> {
        Box::pin(async move {
            let current = self.get_engine_version(settings).await;
            let (latest_tag, _, _) = fetch_latest_ytdlp_release().await?;
            let update_available = match &current {
                Some(cur) => cur.trim() != latest_tag.trim(),
                None => true,
            };
            Ok(EngineUpdateInfo {
                engine: "yt-dlp".to_string(),
                current_version: current,
                latest_version: latest_tag,
                channel: settings.ytdlp_channel.clone(),
                update_available,
                release_date: None,
            })
        })
    }
}
