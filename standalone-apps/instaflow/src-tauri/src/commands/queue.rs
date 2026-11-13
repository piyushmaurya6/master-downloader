use crate::core::url_normalizer::{normalize_instagram_url, sanitize_windows_component};
use crate::db::{JobRecord, NewJobRequest};
use crate::error::AppResult;
use crate::AppState;
use chrono::Utc;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[tauri::command]
pub async fn enqueue_job(
    request: NewJobRequest,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<JobRecord> {
    let norm = normalize_instagram_url(&request.source_url)?;
    let settings = state.repo.get_settings()?;

    let indices = if request.selected_indices.is_empty() {
        norm.img_index_hint.map(|i| vec![i]).unwrap_or_default()
    } else {
        request.selected_indices
    };

    let job = JobRecord {
        id: Uuid::new_v4().to_string(),
        source_url: norm.canonical_url,
        shortcode: request.shortcode.or(norm.shortcode),
        url_type: if request.url_type.trim().is_empty() {
            norm.url_type.kind_slug().to_string()
        } else {
            request.url_type
        },
        author_username: request.author_username.or(norm.username_hint),
        caption: request.caption,
        thumbnail_url: request.thumbnail_url,
        status: "queued".to_string(),
        selected_indices: indices,
        quality_preset: if request.quality_preset.trim().is_empty() {
            settings.default_quality.clone()
        } else {
            request.quality_preset
        },
        output_dir: request.output_dir.unwrap_or(settings.output_dir),
        naming_template: request.naming_template.unwrap_or(settings.naming_template),
        save_caption: request
            .save_caption
            .unwrap_or(settings.save_caption_by_default),
        save_metadata: request
            .save_metadata
            .unwrap_or(settings.save_metadata_by_default),
        embed_thumbnail: request
            .embed_thumbnail
            .unwrap_or(settings.embed_thumbnail_by_default),
        error_message: None,
        progress_percent: 0.0,
        downloaded_bytes: 0,
        total_bytes: None,
        created_at: Utc::now().timestamp_millis(),
        completed_at: None,
        media_files: Vec::new(),
    };

    state.repo.insert_job(&job)?;
    state.queue_worker.notify_new_work();
    let _ = app.emit("queue-updated", &job.id);
    Ok(job)
}

#[tauri::command]
pub async fn enqueue_batch_jobs(
    urls: Vec<String>,
    quality_preset: String,
    subdirectory: Option<String>,
    naming_template: Option<String>,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<Vec<JobRecord>> {
    let settings = state.repo.get_settings()?;
    let base_output_dir = if let Some(ref sub) = subdirectory {
        let trimmed = sub.trim();
        if !trimmed.is_empty() {
            PathBuf::from(&settings.output_dir)
                .join(sanitize_windows_component(trimmed))
                .to_string_lossy()
                .to_string()
        } else {
            settings.output_dir.clone()
        }
    } else {
        settings.output_dir.clone()
    };

    let tpl = naming_template.unwrap_or_else(|| settings.naming_template.clone());
    let mut created_jobs = Vec::new();
    let base_ts = Utc::now().timestamp_millis();

    for (idx, raw_url) in urls.iter().enumerate() {
        if let Ok(norm) = normalize_instagram_url(raw_url) {
            let job = JobRecord {
                id: Uuid::new_v4().to_string(),
                source_url: norm.canonical_url,
                shortcode: norm.shortcode,
                url_type: norm.url_type.kind_slug().to_string(),
                author_username: norm.username_hint,
                caption: None,
                thumbnail_url: None,
                status: "queued".to_string(),
                selected_indices: norm.img_index_hint.map(|i| vec![i]).unwrap_or_default(),
                quality_preset: quality_preset.clone(),
                output_dir: base_output_dir.clone(),
                naming_template: tpl.clone(),
                save_caption: settings.save_caption_by_default,
                save_metadata: settings.save_metadata_by_default,
                embed_thumbnail: settings.embed_thumbnail_by_default,
                error_message: None,
                progress_percent: 0.0,
                downloaded_bytes: 0,
                total_bytes: None,
                created_at: base_ts + (idx as i64),
                completed_at: None,
                media_files: Vec::new(),
            };
            state.repo.insert_job(&job)?;
            created_jobs.push(job);
        }
    }

    state.queue_worker.notify_new_work();
    let _ = app.emit("queue-updated", "batch");
    Ok(created_jobs)
}

#[tauri::command]
pub async fn list_queue(state: State<'_, AppState>) -> AppResult<Vec<JobRecord>> {
    state.repo.list_queue_jobs()
}

#[tauri::command]
pub async fn pause_job(
    job_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    state.queue_worker.pause_job(&job_id);
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}

#[tauri::command]
pub async fn resume_job(
    job_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    state
        .repo
        .update_job_status(&job_id, "queued", None, None)?;
    state.queue_worker.reset_circuit_breaker();
    state.queue_worker.notify_new_work();
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}

#[tauri::command]
pub async fn cancel_job(
    job_id: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    state.queue_worker.cancel_job(&job_id);
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}

#[tauri::command]
pub async fn pause_all_jobs(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<usize> {
    let jobs = state.repo.list_queue_jobs()?;
    let mut count = 0usize;
    for j in jobs {
        if j.status == "running" || j.status == "queued" {
            state.queue_worker.pause_job(&j.id);
            count += 1;
        }
    }
    let _ = app.emit("queue-updated", "pause-all");
    Ok(count)
}

#[tauri::command]
pub async fn resume_all_jobs(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<usize> {
    let count = state.repo.resume_all_paused()?;
    state.queue_worker.reset_circuit_breaker();
    state.queue_worker.notify_new_work();
    let _ = app.emit("queue-updated", "resume-all");
    Ok(count)
}

#[tauri::command]
pub async fn cancel_all_jobs(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<()> {
    let jobs = state.repo.list_queue_jobs()?;
    for j in jobs {
        if j.status == "running" || j.status == "queued" || j.status == "paused" {
            state.queue_worker.cancel_job(&j.id);
        }
    }
    let _ = app.emit("queue-updated", "cancel-all");
    Ok(())
}

#[tauri::command]
pub async fn retry_failed_jobs(
    state: State<'_, AppState>,
    app: AppHandle,
) -> AppResult<usize> {
    let count = state.repo.retry_failed_jobs()?;
    state.queue_worker.reset_circuit_breaker();
    state.queue_worker.notify_new_work();
    let _ = app.emit("queue-updated", "retry-failed");
    Ok(count)
}

#[tauri::command]
pub async fn reorder_queue_job(
    job_id: String,
    new_created_at: i64,
    state: State<'_, AppState>,
) -> AppResult<()> {
    state.repo.reorder_job(&job_id, new_created_at)
}
