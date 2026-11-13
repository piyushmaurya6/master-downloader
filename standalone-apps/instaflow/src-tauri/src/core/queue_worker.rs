use crate::core::binary_manager::ProcessRegistry;
use crate::core::engine_router::{EngineRouter, MediaEngine};
use crate::core::parser::{JobStatus, ProgressPayload};
use crate::db::Repository;
use crate::error::{redact_sensitive, AppError};
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitTrippedPayload {
    pub reason: String,
    pub cooldown_seconds: u64,
    pub requires_reauth: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticLogEntry {
    pub timestamp: String,
    pub level: String,
    pub component: String,
    pub message: String,
}

#[derive(Clone, Default)]
pub struct DiagnosticLogBuffer {
    entries: Arc<Mutex<VecDeque<DiagnosticLogEntry>>>,
}

impl DiagnosticLogBuffer {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(300))),
        }
    }

    pub fn push(&self, level: &str, component: &str, message: &str) {
        let sanitized = redact_sensitive(message);
        let entry = DiagnosticLogEntry {
            timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string(),
            level: level.to_uppercase(),
            component: component.to_string(),
            message: sanitized,
        };
        if let Ok(mut guard) = self.entries.lock() {
            if guard.len() >= 300 {
                guard.pop_front();
            }
            guard.push_back(entry);
        }
    }

    pub fn snapshot(&self) -> Vec<DiagnosticLogEntry> {
        if let Ok(guard) = self.entries.lock() {
            guard.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.entries.lock() {
            guard.clear();
        }
    }
}

#[derive(Clone)]
pub struct QueueWorker {
    repo: Repository,
    router: EngineRouter,
    process_registry: ProcessRegistry,
    log_buffer: DiagnosticLogBuffer,
    notify: Arc<Notify>,
    circuit_breaker_tripped: Arc<AtomicBool>,
    active_workers: Arc<AtomicUsize>,
}

impl QueueWorker {
    pub fn new(
        repo: Repository,
        router: EngineRouter,
        process_registry: ProcessRegistry,
        log_buffer: DiagnosticLogBuffer,
    ) -> Self {
        Self {
            repo,
            router,
            process_registry,
            log_buffer,
            notify: Arc::new(Notify::new()),
            circuit_breaker_tripped: Arc::new(AtomicBool::new(false)),
            active_workers: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn notify_new_work(&self) {
        self.notify.notify_waiters();
    }

    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker_tripped.store(false, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_circuit_breaker_tripped(&self) -> bool {
        self.circuit_breaker_tripped.load(Ordering::SeqCst)
    }

    pub fn cancel_job(&self, job_id: &str) {
        self.process_registry.kill_for_job(job_id);
        let _ = self
            .repo
            .update_job_status(job_id, "cancelled", Some("Cancelled by user"), None);
        self.log_buffer.push(
            "INFO",
            "QueueWorker",
            &format!("Job {} cancelled and child processes terminated.", job_id),
        );
    }

    pub fn pause_job(&self, job_id: &str) {
        self.process_registry.kill_for_job(job_id);
        let _ = self.repo.update_job_status(job_id, "paused", None, None);
        self.log_buffer
            .push("INFO", "QueueWorker", &format!("Job {} paused.", job_id));
    }

    /// Spawns the background Tokio scheduler actor that dispatches queued jobs up to the configured concurrency limit
    /// with randomized anti-ban jitter (`2000ms..=5000ms`) and rate-limit circuit breaker protection.
    pub fn spawn_scheduler(&self, app: AppHandle) {
        let worker = self.clone();
        tauri::async_runtime::spawn(async move {
            worker.log_buffer.push(
                "INFO",
                "QueueWorker",
                "Tokio queue scheduler actor initialized.",
            );

            loop {
                tokio::select! {
                    _ = worker.notify.notified() => {}
                    _ = tokio::time::sleep(Duration::from_millis(1200)) => {}
                }

                if worker.circuit_breaker_tripped.load(Ordering::SeqCst) {
                    continue;
                }

                let settings = match worker.repo.get_settings() {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let max_concurrency = settings.concurrency.clamp(1, 3);
                let current_active = worker.active_workers.load(Ordering::SeqCst);
                if current_active >= max_concurrency {
                    continue;
                }

                let next_job = match worker.repo.get_next_queued_job() {
                    Ok(Some(job)) => job,
                    _ => continue,
                };

                // Mark job as running immediately to prevent duplicate dispatch
                if worker
                    .repo
                    .update_job_status(&next_job.id, "running", None, None)
                    .is_err()
                {
                    continue;
                }

                let _ = app.emit("queue-updated", &next_job.id);
                worker.active_workers.fetch_add(1, Ordering::SeqCst);

                let task_worker = worker.clone();
                let task_app = app.clone();
                let task_settings = settings.clone();

                tauri::async_runtime::spawn(async move {
                    // Enforce randomized anti-ban jitter before dispatching metadata/download requests
                    let jitter_ms = {
                        let min_j = task_settings.jitter_min_ms.max(500);
                        let max_j = task_settings.jitter_max_ms.max(min_j + 200);
                        rand::thread_rng().gen_range(min_j..=max_j)
                    };
                    tokio::time::sleep(Duration::from_millis(jitter_ms)).await;

                    // Re-check job wasn't paused or cancelled during jitter sleep
                    if let Ok(Some(fresh)) = task_worker.repo.get_job(&next_job.id) {
                        if fresh.status != "running" {
                            task_worker.active_workers.fetch_sub(1, Ordering::SeqCst);
                            return;
                        }
                    }

                    task_worker.log_buffer.push(
                        "INFO",
                        "QueueWorker",
                        &format!(
                            "Starting download job {} ({})",
                            next_job.id, next_job.source_url
                        ),
                    );

                    match task_worker
                        .router
                        .download_job(&next_job, &task_settings, Some(&task_app))
                        .await
                    {
                        Ok(media_files) => {
                            if let Some(ref sc) = next_job.shortcode {
                                if let Some(insp) = task_worker.router.get_cached_inspection(sc) {
                                    let _ = task_worker.repo.update_job_metadata(
                                        &next_job.id,
                                        Some(&insp.owner_username),
                                        if insp.caption.is_empty() {
                                            None
                                        } else {
                                            Some(&insp.caption)
                                        },
                                        insp.thumbnail_url.as_deref(),
                                        insp.shortcode.as_deref(),
                                    );
                                }
                            }

                            let mut total_bytes = 0u64;
                            for mf in &media_files {
                                total_bytes += mf.file_size_bytes.unwrap_or(0);
                                let _ = task_worker.repo.insert_media_file(mf);
                            }

                            let now = Utc::now().timestamp();
                            let _ = task_worker.repo.update_job_progress(
                                &next_job.id,
                                100.0,
                                total_bytes,
                                Some(total_bytes),
                            );
                            let _ = task_worker.repo.update_job_status(
                                &next_job.id,
                                "completed",
                                None,
                                Some(now),
                            );

                            let _ = task_app.emit(
                                "download-progress",
                                ProgressPayload {
                                    job_id: next_job.id.clone(),
                                    item_index: media_files.len().max(1),
                                    total_items: media_files.len().max(1),
                                    status: JobStatus::Completed,
                                    stage_label: "Completed".to_string(),
                                    percentage: 100.0,
                                    downloaded_bytes: total_bytes,
                                    total_bytes: Some(total_bytes),
                                    speed_bps: None,
                                    eta_seconds: Some(0),
                                },
                            );

                            let _ = task_app.emit("queue-updated", &next_job.id);
                            task_worker.log_buffer.push(
                                "INFO",
                                "QueueWorker",
                                &format!(
                                    "Job {} completed ({} files, {} bytes).",
                                    next_job.id,
                                    media_files.len(),
                                    total_bytes
                                ),
                            );
                        }
                        Err(err) => {
                            let serialized = err.to_serialized();
                            task_worker.log_buffer.push(
                                "ERROR",
                                "QueueWorker",
                                &format!("Job {} failed: {}", next_job.id, serialized.user_message),
                            );

                            match err {
                                AppError::RateLimited { retry_after_secs } => {
                                    // Trip Global Circuit Breaker
                                    task_worker
                                        .circuit_breaker_tripped
                                        .store(true, Ordering::SeqCst);
                                    let _ = task_worker.repo.pause_all_queued();
                                    let _ = task_worker.repo.update_job_status(
                                        &next_job.id,
                                        "paused",
                                        Some(&serialized.user_message),
                                        None,
                                    );
                                    let _ = task_app.emit(
                                        "rate-limit-tripped",
                                        RateLimitTrippedPayload {
                                            reason: serialized.user_message.clone(),
                                            cooldown_seconds: retry_after_secs.unwrap_or(180),
                                            requires_reauth: false,
                                            timestamp: Utc::now().timestamp(),
                                        },
                                    );
                                }
                                AppError::AuthenticationRequired => {
                                    task_worker
                                        .circuit_breaker_tripped
                                        .store(true, Ordering::SeqCst);
                                    let _ = task_worker.repo.pause_all_queued();
                                    let _ = task_worker.repo.update_job_status(
                                        &next_job.id,
                                        "paused",
                                        Some(&serialized.user_message),
                                        None,
                                    );
                                    let _ = task_app.emit(
                                        "rate-limit-tripped",
                                        RateLimitTrippedPayload {
                                            reason: serialized.user_message.clone(),
                                            cooldown_seconds: 0,
                                            requires_reauth: true,
                                            timestamp: Utc::now().timestamp(),
                                        },
                                    );
                                }
                                _ => {
                                    let _ = task_worker.repo.update_job_status(
                                        &next_job.id,
                                        "failed",
                                        Some(&serialized.user_message),
                                        None,
                                    );
                                    let _ = task_app.emit(
                                        "download-progress",
                                        ProgressPayload {
                                            job_id: next_job.id.clone(),
                                            item_index: 1,
                                            total_items: 1,
                                            status: JobStatus::Failed,
                                            stage_label: serialized.user_message,
                                            percentage: 0.0,
                                            downloaded_bytes: 0,
                                            total_bytes: None,
                                            speed_bps: None,
                                            eta_seconds: None,
                                        },
                                    );
                                }
                            }
                            let _ = task_app.emit("queue-updated", &next_job.id);
                        }
                    }

                    task_worker.active_workers.fetch_sub(1, Ordering::SeqCst);
                    task_worker.notify.notify_waiters();
                });
            }
        });
    }
}
