use crate::dependencies::ffmpeg_service::detect_ffmpeg_tool;
use crate::dependencies::ytdlp_service::detect_ytdlp;
use crate::downloader::download_manager::build_ytdlp_arguments;
use crate::downloader::progress_parser::{parse_ytdlp_line, OutputLineEvent};
use crate::models::{DownloadEventPayload, DownloadItem, DownloadRequest, HistoryItem};
use crate::platform::validate_safe_output_dir;
use crate::services::error_handler::classify_error;
use crate::storage::history_store::HistoryStore;
use crate::storage::settings_store::SettingsStore;
use crate::utils::{no_window_command, DiagnosticLogger};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Clone)]
pub struct QueueManager {
    items: Arc<Mutex<Vec<DownloadItem>>>,
    cancel_senders: Arc<Mutex<HashMap<String, (watch::Sender<bool>, String)>>>, // id -> (sender, "cancelled"|"stopped")
    settings_store: SettingsStore,
    history_store: HistoryStore,
    logger: DiagnosticLogger,
}

impl QueueManager {
    pub fn new(
        settings_store: SettingsStore,
        history_store: HistoryStore,
        logger: DiagnosticLogger,
    ) -> Self {
        let restored = history_store.load_queue().unwrap_or_default();
        Self {
            items: Arc::new(Mutex::new(restored)),
            cancel_senders: Arc::new(Mutex::new(HashMap::new())),
            settings_store,
            history_store,
            logger,
        }
    }

    pub fn list_queue(&self) -> Vec<DownloadItem> {
        self.items
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn active_count(&self) -> usize {
        self.items
            .lock()
            .map(|g| {
                g.iter()
                    .filter(|i| {
                        i.status == "downloading"
                            || i.status == "analyzing"
                            || i.status == "postprocessing"
                            || i.status == "retrying"
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    fn persist_and_emit(&self, app: Option<&AppHandle>) {
        let snapshot = self.list_queue();
        let _ = self.history_store.save_queue(&snapshot);
        if let Some(handle) = app {
            let _ = handle.emit("queue-updated", &snapshot);
        }
    }

    fn emit_contract_event(&self, app: Option<&AppHandle>, event_name: &str, item: &DownloadItem) {
        if let Some(handle) = app {
            let payload = DownloadEventPayload {
                download_id: item.id.clone(),
                state: item.status.clone(),
                progress: item.progress,
                downloaded_bytes: item.downloaded_bytes,
                total_bytes: item.total_bytes,
                speed_bytes_per_sec: item.speed_bytes_per_sec,
                eta_seconds: item.eta_seconds,
                current_operation: item.postprocess_stage.clone(),
                output_path: item.output_path.clone(),
                error_code: item.error_category.clone(),
                error_message: item.error_message.clone(),
            };
            let _ = handle.emit(event_name, &payload);
        }
    }

    pub fn enqueue(
        &self,
        app: Option<&AppHandle>,
        req: DownloadRequest,
    ) -> Result<DownloadItem, String> {
        let settings = self.settings_store.get();
        let (_, format_summary, quality_label) = build_ytdlp_arguments(&req, &settings, None)?;

        let raw_dir = req
            .output_dir
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| settings.download_dir.clone());
        let output_dir = validate_safe_output_dir(&raw_dir)?;

        let item = DownloadItem {
            id: Uuid::new_v4().to_string(),
            url: req.url.clone(),
            title: req.title.clone(),
            thumbnail: req.thumbnail.clone(),
            uploader: req.uploader.clone(),
            duration: req.duration,
            extractor: req.extractor.clone(),
            mode: req.mode.clone(),
            format_summary,
            quality_label,
            status: "queued".to_string(),
            postprocess_stage: None,
            progress: 0.0,
            downloaded_bytes: 0,
            total_bytes: None,
            speed_bytes_per_sec: None,
            eta_seconds: None,
            output_path: None,
            output_dir,
            error_message: None,
            technical_error: None,
            error_category: None,
            recovery_action: None,
            suggest_engine_update: false,
            retry_count: 0,
            max_retries: settings.retry_count,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
            request: req,
        };

        if let Ok(mut guard) = self.items.lock() {
            guard.insert(0, item.clone());
        }
        self.emit_contract_event(app, "download:queued", &item);
        self.persist_and_emit(app);
        self.schedule_next(app.cloned());
        Ok(item)
    }

    pub fn cancel_or_stop(
        &self,
        app: Option<&AppHandle>,
        id: &str,
        target_state: &str, // "cancelled" | "stopped"
    ) -> Result<(), String> {
        let mut was_running = false;
        if let Ok(mut senders) = self.cancel_senders.lock() {
            if let Some((tx, state_label)) = senders.get_mut(id) {
                *state_label = target_state.to_string();
                let _ = tx.send(true);
                was_running = true;
            }
        }

        if !was_running {
            let mut updated_opt = None;
            if let Ok(mut guard) = self.items.lock() {
                if let Some(item) = guard.iter_mut().find(|i| i.id == id) {
                    item.status = target_state.to_string();
                    item.speed_bytes_per_sec = None;
                    item.eta_seconds = None;
                    item.postprocess_stage = None;
                    updated_opt = Some(item.clone());
                }
            }
            if let Some(ref it) = updated_opt {
                self.emit_contract_event(app, "download:cancelled", it);
            }
            self.persist_and_emit(app);
        }
        Ok(())
    }

    pub fn resume_or_retry(&self, app: Option<&AppHandle>, id: &str) -> Result<(), String> {
        if let Ok(mut guard) = self.items.lock() {
            if let Some(item) = guard.iter_mut().find(|i| i.id == id) {
                item.status = "queued".to_string();
                item.error_message = None;
                item.technical_error = None;
                item.error_category = None;
                item.recovery_action = None;
                item.suggest_engine_update = false;
                item.retry_count = 0;
                item.speed_bytes_per_sec = None;
                item.eta_seconds = None;
            }
        }
        self.persist_and_emit(app);
        self.schedule_next(app.cloned());
        Ok(())
    }

    pub fn remove_item(&self, app: Option<&AppHandle>, id: &str) -> Result<(), String> {
        let _ = self.cancel_or_stop(app, id, "cancelled");
        if let Ok(mut guard) = self.items.lock() {
            guard.retain(|i| i.id != id);
        }
        self.persist_and_emit(app);
        Ok(())
    }

    pub fn clear_completed(&self, app: Option<&AppHandle>) -> Result<(), String> {
        if let Ok(mut guard) = self.items.lock() {
            guard.retain(|i| i.status != "completed" && i.status != "cancelled");
        }
        self.persist_and_emit(app);
        Ok(())
    }

    pub fn reorder_item(
        &self,
        app: Option<&AppHandle>,
        id: &str,
        direction: &str, // "up" | "down"
    ) -> Result<(), String> {
        if let Ok(mut guard) = self.items.lock() {
            if let Some(idx) = guard.iter().position(|i| i.id == id) {
                if direction == "up" && idx > 0 {
                    guard.swap(idx, idx - 1);
                } else if direction == "down" && idx + 1 < guard.len() {
                    guard.swap(idx, idx + 1);
                }
            }
        }
        self.persist_and_emit(app);
        Ok(())
    }

    pub fn schedule_next(&self, app: Option<AppHandle>) {
        let max_concurrent = self.settings_store.get().max_concurrent_downloads.max(1);

        loop {
            let next_id: Option<String> = {
                let mut guard = match self.items.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                let active = guard
                    .iter()
                    .filter(|i| {
                        i.status == "downloading"
                            || i.status == "analyzing"
                            || i.status == "postprocessing"
                            || i.status == "retrying"
                    })
                    .count();

                if active >= max_concurrent {
                    None
                } else {
                    // Pick oldest queued item (FIFO from bottom of queued list)
                    guard
                        .iter_mut()
                        .rev()
                        .find(|i| i.status == "queued")
                        .map(|item| {
                            item.status = "analyzing".to_string();
                            item.postprocess_stage = Some("downloading_video".to_string());
                            item.id.clone()
                        })
                }
            };

            match next_id {
                Some(id) => {
                    self.persist_and_emit(app.as_ref());
                    let mgr = self.clone();
                    let app_clone = app.clone();
                    tokio::spawn(async move {
                        mgr.execute_download_job(app_clone, id).await;
                    });
                }
                None => break,
            }
        }
    }

    async fn execute_download_job(&self, app: Option<AppHandle>, id: String) {
        let item_snapshot = {
            let guard = self.items.lock().ok();
            guard.and_then(|g| g.iter().find(|i| i.id == id).cloned())
        };
        let Some(item) = item_snapshot else {
            return;
        };

        let settings = self.settings_store.get();
        let ytdlp_status = detect_ytdlp(
            settings.ytdlp_path.as_deref(),
            settings.prefer_managed_binaries,
        )
        .await;

        let Some(ytdlp_path_str) = ytdlp_status.path else {
            self.mark_failed(
                app.as_ref(),
                &id,
                "DEPENDENCY_MISSING",
                "yt-dlp download engine is not installed. Run repair in Settings.",
                "yt-dlp executable not found",
                Some("Open Settings -> Dependency Manager and click Repair."),
                false,
            );
            self.schedule_next(app);
            return;
        };

        let ffmpeg_status = detect_ffmpeg_tool(
            "ffmpeg",
            "ffmpeg.exe",
            settings.ffmpeg_path.as_deref(),
            settings.prefer_managed_binaries,
        )
        .await;

        let ffmpeg_dir: Option<PathBuf> = ffmpeg_status
            .path
            .as_ref()
            .and_then(|p| Path::new(p).parent().map(|d| d.to_path_buf()));

        // Ensure output directory exists
        let _ = std::fs::create_dir_all(&item.output_dir);

        let args_res = build_ytdlp_arguments(&item.request, &settings, ffmpeg_dir.as_deref());
        let (args, _, _) = match args_res {
            Ok(v) => v,
            Err(e) => {
                self.mark_failed(app.as_ref(), &id, "INVALID_URL", &e, &e, None, false);
                self.schedule_next(app);
                return;
            }
        };

        let max_attempts = item.max_retries + 1;
        let mut attempt = 0;

        while attempt < max_attempts {
            attempt += 1;
            let (tx, mut rx) = watch::channel(false);
            if let Ok(mut senders) = self.cancel_senders.lock() {
                senders.insert(id.clone(), (tx, "cancelled".to_string()));
            }

            let mut started_item_opt = None;
            if let Ok(mut guard) = self.items.lock() {
                if let Some(it) = guard.iter_mut().find(|i| i.id == id) {
                    it.status = if attempt > 1 {
                        "retrying".to_string()
                    } else {
                        "downloading".to_string()
                    };
                    it.retry_count = attempt.saturating_sub(1);
                    it.postprocess_stage = Some(if it.mode == "audio" {
                        "downloading_audio".to_string()
                    } else {
                        "downloading_video".to_string()
                    });
                    started_item_opt = Some(it.clone());
                }
            }
            if let Some(ref st) = started_item_opt {
                self.emit_contract_event(app.as_ref(), "download:started", st);
            }
            self.persist_and_emit(app.as_ref());

            self.logger.info(
                "DownloadEngine",
                &format!("Starting download job {} (attempt {})", id, attempt),
            );

            let mut child = match no_window_command(&ytdlp_path_str)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    self.mark_failed(
                        app.as_ref(),
                        &id,
                        "UNKNOWN_ERROR",
                        "Failed to start download engine process.",
                        &e.to_string(),
                        Some("Verify engine health in Settings."),
                        false,
                    );
                    break;
                }
            };

            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            let mut captured_filepath: Option<String> = None;
            let mut error_lines: Vec<String> = Vec::new();
            let mut last_emit = Instant::now();
            let mut user_interrupted = false;

            loop {
                tokio::select! {
                    _ = rx.changed() => {
                        if *rx.borrow() {
                            user_interrupted = true;
                            let _ = child.kill().await;
                            break;
                        }
                    }
                    line_res = stdout_reader.next_line() => {
                        match line_res {
                            Ok(Some(line)) => {
                                self.handle_process_line(
                                    &id,
                                    &line,
                                    &mut captured_filepath,
                                    &mut error_lines,
                                    &mut last_emit,
                                    app.as_ref(),
                                );
                            }
                            Ok(None) => break,
                            Err(_) => break,
                        }
                    }
                    err_res = stderr_reader.next_line() => {
                        if let Ok(Some(line)) = err_res {
                            if !line.trim().is_empty() {
                                error_lines.push(line.clone());
                            }
                            self.handle_process_line(
                                &id,
                                &line,
                                &mut captured_filepath,
                                &mut error_lines,
                                &mut last_emit,
                                app.as_ref(),
                            );
                        }
                    }
                }
            }

            let status_res = child.wait().await;
            let interrupt_type = self
                .cancel_senders
                .lock()
                .ok()
                .and_then(|mut s| s.remove(&id).map(|(_, t)| t))
                .unwrap_or_else(|| "cancelled".to_string());

            if user_interrupted {
                let mut cancelled_opt = None;
                if let Ok(mut guard) = self.items.lock() {
                    if let Some(it) = guard.iter_mut().find(|i| i.id == id) {
                        it.status = interrupt_type.clone();
                        it.speed_bytes_per_sec = None;
                        it.eta_seconds = None;
                        it.postprocess_stage = None;
                        cancelled_opt = Some(it.clone());
                    }
                }
                if let Some(ref c_item) = cancelled_opt {
                    self.emit_contract_event(app.as_ref(), "download:cancelled", c_item);
                }
                self.logger.info(
                    "DownloadEngine",
                    &format!("Download {} was {} by user.", id, interrupt_type),
                );
                self.persist_and_emit(app.as_ref());
                break;
            }

            let exited_ok = status_res.map(|s| s.success()).unwrap_or(false);
            if exited_ok {
                // Verify output file and record completion only after postprocessing succeeds
                let final_path = captured_filepath.clone();
                let file_size = final_path
                    .as_ref()
                    .and_then(|p| std::fs::metadata(p).ok().map(|m| m.len()));

                let mut completed_item_opt: Option<DownloadItem> = None;
                if let Ok(mut guard) = self.items.lock() {
                    if let Some(it) = guard.iter_mut().find(|i| i.id == id) {
                        it.status = "completed".to_string();
                        it.progress = 100.0;
                        it.postprocess_stage = None;
                        it.speed_bytes_per_sec = None;
                        it.eta_seconds = Some(0);
                        if let Some(sz) = file_size {
                            it.downloaded_bytes = sz;
                            it.total_bytes = Some(sz);
                        }
                        it.output_path = final_path.clone();
                        it.completed_at = Some(Utc::now().to_rfc3339());
                        completed_item_opt = Some(it.clone());
                    }
                }

                if let Some(done) = completed_item_opt {
                    self.emit_contract_event(app.as_ref(), "download:completed", &done);
                    let hist = HistoryItem {
                        id: done.id.clone(),
                        title: done.title.clone(),
                        thumbnail: done.thumbnail.clone(),
                        uploader: done.uploader.clone(),
                        source_url: done.url.clone(),
                        extractor: done.extractor.clone(),
                        downloaded_at: done.completed_at.clone().unwrap_or_else(|| Utc::now().to_rfc3339()),
                        mode: done.mode.clone(),
                        format_summary: done.format_summary.clone(),
                        quality: done.quality_label.clone(),
                        file_size,
                        output_path: done.output_path.clone(),
                        output_dir: done.output_dir.clone(),
                        status: "completed".to_string(),
                        file_exists: true,
                        error_message: None,
                        request: done.request.clone(),
                    };
                    let _ = self.history_store.add_history(&hist);
                }

                self.logger
                    .info("DownloadEngine", &format!("Download {} completed.", id));
                self.persist_and_emit(app.as_ref());
                break;
            } else {
                let raw_err = error_lines.join("\n");
                let classified = classify_error(&raw_err);
                if classified.retryable && !classified.suggest_engine_update && attempt < max_attempts {
                    self.logger.warn(
                        "DownloadEngine",
                        &format!(
                            "Transient error on download {}, retrying ({}/{}): {}",
                            id, attempt, item.max_retries, classified.technical_details
                        ),
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                } else {
                    self.mark_failed(
                        app.as_ref(),
                        &id,
                        &classified.code,
                        &classified.user_message,
                        &classified.technical_details,
                        Some(&classified.recovery_action),
                        classified.suggest_engine_update,
                    );
                    break;
                }
            }
        }

        self.schedule_next(app);
    }

    fn handle_process_line(
        &self,
        id: &str,
        line: &str,
        captured_filepath: &mut Option<String>,
        error_lines: &mut Vec<String>,
        last_emit: &mut Instant,
        app: Option<&AppHandle>,
    ) {
        match parse_ytdlp_line(line) {
            OutputLineEvent::Progress(prog) => {
                let mut prog_item_opt = None;
                if let Ok(mut guard) = self.items.lock() {
                    if let Some(it) = guard.iter_mut().find(|i| i.id == id) {
                        it.status = "downloading".to_string();
                        it.progress = prog.percent;
                        if prog.downloaded_bytes > 0 {
                            it.downloaded_bytes = prog.downloaded_bytes;
                        }
                        if prog.total_bytes.is_some() {
                            it.total_bytes = prog.total_bytes;
                        }
                        it.speed_bytes_per_sec = prog.speed_bytes_per_sec;
                        it.eta_seconds = prog.eta_seconds;
                        prog_item_opt = Some(it.clone());
                    }
                }
                if last_emit.elapsed() >= Duration::from_millis(120) {
                    *last_emit = Instant::now();
                    if let Some(ref p_it) = prog_item_opt {
                        self.emit_contract_event(app, "download:progress", p_it);
                    }
                    if let Some(handle) = app {
                        let _ = handle.emit("queue-updated", &self.list_queue());
                    }
                }
            }
            OutputLineEvent::PostprocessStage(stage) => {
                let mut post_item_opt = None;
                if let Ok(mut guard) = self.items.lock() {
                    if let Some(it) = guard.iter_mut().find(|i| i.id == id) {
                        if stage == "merging_streams"
                            || stage == "converting_audio"
                            || stage == "embedding_metadata"
                            || stage == "finalizing"
                        {
                            it.status = "postprocessing".to_string();
                            it.progress = it.progress.max(99.0);
                            it.speed_bytes_per_sec = None;
                            it.eta_seconds = None;
                        }
                        it.postprocess_stage = Some(stage);
                        post_item_opt = Some(it.clone());
                    }
                }
                if let Some(ref p_it) = post_item_opt {
                    self.emit_contract_event(app, "download:postprocessing", p_it);
                }
                self.persist_and_emit(app);
            }
            OutputLineEvent::DestinationFile(dest) => {
                if captured_filepath.is_none() {
                    *captured_filepath = Some(dest);
                }
            }
            OutputLineEvent::FinalFilepath(fp) => {
                *captured_filepath = Some(fp);
            }
            OutputLineEvent::ErrorLine(err) => {
                error_lines.push(err);
            }
            OutputLineEvent::InfoLine => {}
        }
    }

    fn mark_failed(
        &self,
        app: Option<&AppHandle>,
        id: &str,
        category: &str,
        user_msg: &str,
        tech_msg: &str,
        recovery_action: Option<&str>,
        suggest_engine_update: bool,
    ) {
        self.logger.error(
            "DownloadEngine",
            &format!("Download {} failed ({}): {}", id, category, tech_msg),
        );
        let mut failed_item_opt = None;
        if let Ok(mut guard) = self.items.lock() {
            if let Some(it) = guard.iter_mut().find(|i| i.id == id) {
                it.status = "failed".to_string();
                it.postprocess_stage = None;
                it.speed_bytes_per_sec = None;
                it.eta_seconds = None;
                it.error_category = Some(category.to_string());
                it.error_message = Some(user_msg.to_string());
                it.technical_error = Some(tech_msg.to_string());
                it.recovery_action = recovery_action.map(|s| s.to_string());
                it.suggest_engine_update = suggest_engine_update;
                failed_item_opt = Some(it.clone());
            }
        }
        if let Some(failed) = failed_item_opt {
            self.emit_contract_event(app, "download:failed", &failed);
            let hist = HistoryItem {
                id: failed.id.clone(),
                title: failed.title.clone(),
                thumbnail: failed.thumbnail.clone(),
                uploader: failed.uploader.clone(),
                source_url: failed.url.clone(),
                extractor: failed.extractor.clone(),
                downloaded_at: Utc::now().to_rfc3339(),
                mode: failed.mode.clone(),
                format_summary: failed.format_summary.clone(),
                quality: failed.quality_label.clone(),
                file_size: None,
                output_path: None,
                output_dir: failed.output_dir.clone(),
                status: "failed".to_string(),
                file_exists: false,
                error_message: failed.error_message.clone(),
                request: failed.request.clone(),
            };
            let _ = self.history_store.add_history(&hist);
        }
        self.persist_and_emit(app);
    }
}
