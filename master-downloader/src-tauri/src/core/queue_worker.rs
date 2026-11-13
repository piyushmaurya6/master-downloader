use crate::core::engine_router::EngineRouter;
use crate::core::parser::ProgressPayload;
use crate::db::repository::DbRepository;
use crate::error::AppError;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitTrippedPayload {
    pub job_id: String,
    pub reason: String,
    pub cooldown_seconds: u64,
    pub requires_reauth: bool,
}

#[derive(Clone)]
pub struct QueueWorker {
    db: DbRepository,
    router: EngineRouter,
    notify: Arc<Notify>,
    circuit_breaker_tripped: Arc<AtomicBool>,
}

impl QueueWorker {
    pub fn new(db: DbRepository, router: EngineRouter) -> Self {
        Self {
            db,
            router,
            notify: Arc::new(Notify::new()),
            circuit_breaker_tripped: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn wake(&self) {
        self.notify.notify_waiters();
    }

    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker_tripped.store(false, Ordering::SeqCst);
        self.wake();
    }

    pub fn spawn_background_loop(&self, app: AppHandle) {
        let worker = self.clone();
        tauri::async_runtime::spawn(async move {
            let active_count = Arc::new( std::sync::atomic::AtomicUsize::new(0));

            loop {
                if worker.circuit_breaker_tripped.load(Ordering::SeqCst) {
                    worker.notify.notified().await;
                    continue;
                }

                let settings = worker.db.get_settings().unwrap_or_default();
                let max_concurrency = settings.concurrency.clamp(1, 5);
                let current_active = active_count.load(Ordering::SeqCst);

                if current_active >= max_concurrency {
                    tokio::select! {
                        _ = worker.notify.notified() => {},
                        _ = tokio::time::sleep(Duration::from_millis(500)) => {},
                    }
                    continue;
                }

                match worker.db.claim_next_queued_job() {
                    Ok(Some(job)) => {
                        active_count.fetch_add(1, Ordering::SeqCst);
                        let _ = app.emit("queue-updated", &job.id);

                        let w_clone = worker.clone();
                        let app_clone = app.clone();
                        let counter_clone = active_count.clone();
                        let jitter_min = settings.jitter_min_ms;
                        let jitter_max = settings.jitter_max_ms.max(jitter_min + 100);
                        let organize_by_platform = settings.organize_by_platform;
                        let organize_by_uploader = settings.organize_by_uploader;

                        tauri::async_runtime::spawn(async move {
                            w_clone.router.processes.clear_aborted(&job.id);

                            if current_active > 0 {
                                let delay_ms = {
                                    let mut rng = rand::thread_rng();
                                    rng.gen_range(jitter_min.min(500)..=jitter_max.min(1200))
                                };
                                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            }

                            // Verify job was not paused or cancelled during jitter delay
                            if w_clone.router.processes.is_job_aborted(&job.id) {
                                counter_clone.fetch_sub(1, Ordering::SeqCst);
                                w_clone.wake();
                                return;
                            }
                            if let Ok(Some(latest)) = w_clone.db.get_job(&job.id) {
                                if latest.status == "paused" || latest.status == "cancelled" {
                                    counter_clone.fetch_sub(1, Ordering::SeqCst);
                                    w_clone.wake();
                                    return;
                                }
                            } else {
                                counter_clone.fetch_sub(1, Ordering::SeqCst);
                                w_clone.wake();
                                return;
                            }

                            let mut enriched_job = job.clone();
                            if enriched_job.caption.is_none() || enriched_job.thumbnail_url.is_none() {
                                if let Ok(meta) = w_clone.router.inspect_url(&enriched_job.source_url).await {
                                    let thumb = meta.slides.first().and_then(|s| s.thumbnail_url.clone());
                                    let _ = w_clone.db.update_job_metadata(
                                        &enriched_job.id,
                                        Some(&meta.author_username),
                                        meta.caption.as_deref(),
                                        thumb.as_deref(),
                                    );
                                    enriched_job.author_username = Some(meta.author_username);
                                    if enriched_job.caption.is_none() {
                                        enriched_job.caption = meta.caption;
                                    }
                                    if enriched_job.thumbnail_url.is_none() {
                                        enriched_job.thumbnail_url = thumb;
                                    }
                                    let _ = app_clone.emit("queue-updated", &enriched_job.id);
                                }
                            }

                            let exec_res = w_clone
                                .router
                                .execute_job(
                                    &enriched_job,
                                    organize_by_platform,
                                    organize_by_uploader,
                                    Some(&app_clone),
                                )
                                .await;

                            // Respect user-initiated Pause / Cancel / Delete during execution
                            let current_status = w_clone
                                .db
                                .get_job(&job.id)
                                .ok()
                                .flatten()
                                .map(|j| j.status);
                            if current_status.is_none()
                                || matches!(current_status.as_deref(), Some("paused" | "cancelled"))
                            {
                                let _ = app_clone.emit("queue-updated", &job.id);
                                counter_clone.fetch_sub(1, Ordering::SeqCst);
                                w_clone.wake();
                                return;
                            }

                            match exec_res {
                                Ok(files) => {
                                    let mut total_bytes = 0u64;
                                    for file in &files {
                                        total_bytes += file.file_size_bytes.unwrap_or(0);
                                        let _ = w_clone.db.insert_media_file(file);
                                    }
                                    let _ = w_clone.db.update_job_progress(
                                        &job.id,
                                        100.0,
                                        total_bytes,
                                        Some(total_bytes),
                                    );
                                    let _ = w_clone
                                        .db
                                        .update_job_status(&job.id, "completed", None);

                                    let _ = app_clone.emit(
                                        "download-progress",
                                        ProgressPayload {
                                            job_id: job.id.clone(),
                                            slide_index: files.len().max(1),
                                            status: "completed".into(),
                                            downloaded_bytes: total_bytes,
                                            total_bytes: Some(total_bytes),
                                            speed_bps: None,
                                            eta_seconds: Some(0),
                                            percentage: 100.0,
                                        },
                                    );
                                    let _ = app_clone.emit("queue-updated", &job.id);
                                }
                                Err(err) => {
                                    let serialized = err.to_serialized();
                                    if matches!(err, AppError::RateLimited(_)) {
                                        let _ = w_clone.db.update_job_status(
                                            &job.id,
                                            "failed",
                                            Some(&serialized.user_message),
                                        );
                                        let _ = app_clone.emit(
                                            "rate-limit-tripped",
                                            RateLimitTrippedPayload {
                                                job_id: job.id.clone(),
                                                reason: serialized.user_message.clone(),
                                                cooldown_seconds: 60,
                                                requires_reauth: true,
                                            },
                                        );
                                    } else if matches!(err, AppError::AuthenticationRequired(_)) {
                                        let _ = w_clone.db.update_job_status(
                                            &job.id,
                                            "failed",
                                            Some(&serialized.user_message),
                                        );
                                        let _ = app_clone.emit(
                                            "rate-limit-tripped",
                                            RateLimitTrippedPayload {
                                                job_id: job.id.clone(),
                                                reason: serialized.user_message.clone(),
                                                cooldown_seconds: 0,
                                                requires_reauth: true,
                                            },
                                        );
                                    } else {
                                        let _ = w_clone.db.update_job_status(
                                            &job.id,
                                            "failed",
                                            Some(&serialized.user_message),
                                        );
                                    }

                                    let _ = app_clone.emit("queue-updated", &job.id);
                                }
                            }

                            counter_clone.fetch_sub(1, Ordering::SeqCst);
                            w_clone.wake();
                        });
                    }
                    Ok(None) => {
                        tokio::select! {
                            _ = worker.notify.notified() => {},
                            _ = tokio::time::sleep(Duration::from_secs(2)) => {},
                        }
                    }
                    Err(e) => {
                        tracing::error!("Queue claim error: {}", e);
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        });
    }
}
