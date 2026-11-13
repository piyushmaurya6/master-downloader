use crate::core::url_normalizer::{normalize_media_url, MediaUrlType};
use crate::db::repository::{AdvancedJobOptions, JobRecord, NewJobRequest};
use crate::error::AppResult;
use crate::AppState;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn enqueue_job(
    state: State<'_, AppState>,
    app: AppHandle,
    request: NewJobRequest,
) -> AppResult<JobRecord> {
    let settings = state.db.get_settings()?;
    let norm = normalize_media_url(&request.source_url).ok();
    let mut req = request;
    if req.platform.is_none() {
        req.platform = norm.as_ref().map(|n| n.platform.as_str().to_string());
    }
    if req.shortcode.is_none() {
        req.shortcode = norm.as_ref().and_then(|n| n.shortcode.clone());
    }
    if req.options.speed_limit_kbps.is_none() {
        req.options.speed_limit_kbps = settings.speed_limit_kbps;
    }

    let job = state.db.insert_job(&req, &settings.output_dir)?;
    let _ = app.emit("queue-updated", &job.id);
    state.worker.reset_circuit_breaker();
    Ok(job)
}

/// Instant 1-step Auto-Detect & Auto-Download command:
/// Normalizes any YouTube, Instagram, Facebook, or Generic URL, applies user's default settings,
/// inserts the job into SQLite, and immediately wakes the background worker.
#[tauri::command]
pub async fn auto_enqueue_url(
    state: State<'_, AppState>,
    app: AppHandle,
    url: String,
    quality_override: Option<String>,
) -> AppResult<JobRecord> {
    let norm = normalize_media_url(&url)?;
    let settings = state.db.get_settings()?;

    let quality_preset = quality_override.unwrap_or_else(|| {
        if norm.url_type == MediaUrlType::Music {
            format!("audio_{}", settings.default_audio_format)
        } else {
            settings.default_quality.clone()
        }
    });

    let selected_indices = norm
        .carousel_index_hint
        .map(|idx| vec![idx])
        .unwrap_or_default();

    let req = NewJobRequest {
        source_url: norm.canonical_url,
        platform: Some(norm.platform.as_str().to_string()),
        shortcode: norm.shortcode,
        url_type: norm.url_type.as_str().to_string(),
        author_username: norm.username,
        caption: None,
        thumbnail_url: None,
        selected_indices,
        quality_preset,
        output_dir: Some(settings.output_dir.clone()),
        save_caption: settings.save_caption_sidecar,
        save_metadata: settings.save_metadata_json,
        embed_thumbnail: settings.embed_thumbnail,
        naming_template: Some(settings.naming_template.clone()),
        options: AdvancedJobOptions {
            container: Some(settings.default_container.clone()),
            preferred_codec: Some("auto".into()),
            preferred_fps: None,
            exact_resolution: false,
            audio_format: Some(settings.default_audio_format.clone()),
            audio_bitrate: Some(settings.default_audio_bitrate.clone()),
            audio_normalization: settings.audio_normalization,
            write_subtitles: settings.write_subtitles,
            subtitle_langs: Some("en,.*-orig".into()),
            embed_chapters: settings.embed_chapters,
            playlist_mode: if norm.url_type == MediaUrlType::Playlist {
                Some("playlist".into())
            } else {
                Some("single".into())
            },
            speed_limit_kbps: settings.speed_limit_kbps,
        },
    };

    let job = state.db.insert_job(&req, &settings.output_dir)?;
    let _ = app.emit("queue-updated", &job.id);
    state.worker.reset_circuit_breaker();
    Ok(job)
}

#[tauri::command]
pub async fn enqueue_batch_jobs(
    state: State<'_, AppState>,
    app: AppHandle,
    urls: Vec<String>,
    quality_preset: String,
    subdirectory: Option<String>,
    naming_template: Option<String>,
) -> AppResult<Vec<JobRecord>> {
    let settings = state.db.get_settings()?;
    let base_output_dir = if let Some(ref sub) = subdirectory {
        let trimmed = sub.trim();
        if !trimmed.is_empty() {
            PathBuf::from(&settings.output_dir)
                .join(trimmed)
                .to_string_lossy()
                .to_string()
        } else {
            settings.output_dir.clone()
        }
    } else {
        settings.output_dir.clone()
    };

    let mut created = Vec::new();

    for raw in urls {
        let Ok(norm) = normalize_media_url(&raw) else {
            continue;
        };
        let req = NewJobRequest {
            source_url: norm.canonical_url,
            platform: Some(norm.platform.as_str().to_string()),
            shortcode: norm.shortcode,
            url_type: norm.url_type.as_str().to_string(),
            author_username: norm.username,
            caption: None,
            thumbnail_url: None,
            selected_indices: norm
                .carousel_index_hint
                .map(|idx| vec![idx])
                .unwrap_or_default(),
            quality_preset: quality_preset.clone(),
            output_dir: Some(base_output_dir.clone()),
            save_caption: settings.save_caption_sidecar,
            save_metadata: settings.save_metadata_json,
            embed_thumbnail: settings.embed_thumbnail,
            naming_template: naming_template
                .clone()
                .or_else(|| Some(settings.naming_template.clone())),
            options: AdvancedJobOptions {
                container: Some(settings.default_container.clone()),
                preferred_codec: Some("auto".into()),
                preferred_fps: None,
                exact_resolution: false,
                audio_format: Some(settings.default_audio_format.clone()),
                audio_bitrate: Some(settings.default_audio_bitrate.clone()),
                audio_normalization: settings.audio_normalization,
                write_subtitles: settings.write_subtitles,
                subtitle_langs: Some("en,.*-orig".into()),
                embed_chapters: settings.embed_chapters,
                playlist_mode: if norm.url_type == MediaUrlType::Playlist {
                    Some("playlist".into())
                } else {
                    Some("single".into())
                },
                speed_limit_kbps: settings.speed_limit_kbps,
            },
        };
        if let Ok(job) = state.db.insert_job(&req, &base_output_dir) {
            created.push(job);
        }
    }

    let _ = app.emit("queue-updated", "batch");
    state.worker.reset_circuit_breaker();
    Ok(created)
}

#[tauri::command]
pub async fn list_queue(state: State<'_, AppState>) -> AppResult<Vec<JobRecord>> {
    state.db.list_active_queue()
}

#[tauri::command]
pub async fn pause_job(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: String,
) -> AppResult<()> {
    state.db.update_job_status(&job_id, "paused", None)?;
    state.processes.abort_and_kill_job(&job_id);
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}

#[tauri::command]
pub async fn resume_job(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: String,
) -> AppResult<()> {
    state.processes.clear_aborted(&job_id);
    state.db.update_job_status(&job_id, "queued", None)?;
    state.worker.reset_circuit_breaker();
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}

#[tauri::command]
pub async fn cancel_job(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: String,
) -> AppResult<()> {
    state
        .db
        .update_job_status(&job_id, "cancelled", Some("Cancelled by user"))?;
    state.processes.abort_and_kill_job(&job_id);
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}

#[tauri::command]
pub async fn pause_all_jobs(state: State<'_, AppState>, app: AppHandle) -> AppResult<usize> {
    let count = state.db.pause_all_active_jobs()?;
    state.processes.kill_all();
    let _ = app.emit("queue-updated", "all");
    Ok(count)
}

#[tauri::command]
pub async fn resume_all_jobs(state: State<'_, AppState>, app: AppHandle) -> AppResult<usize> {
    state.processes.clear_all_aborted();
    let count = state.db.resume_all_paused_jobs()?;
    state.worker.reset_circuit_breaker();
    let _ = app.emit("queue-updated", "all");
    Ok(count)
}

#[tauri::command]
pub async fn cancel_all_jobs(state: State<'_, AppState>, app: AppHandle) -> AppResult<()> {
    let active = state.db.list_active_queue()?;
    for j in active {
        if j.status == "running" || j.status == "queued" || j.status == "paused" {
            let _ = state
                .db
                .update_job_status(&j.id, "cancelled", Some("Cancelled by user"));
        }
    }
    state.processes.kill_all();
    let _ = app.emit("queue-updated", "all");
    Ok(())
}

#[tauri::command]
pub async fn retry_failed_jobs(state: State<'_, AppState>, app: AppHandle) -> AppResult<usize> {
    state.processes.clear_all_aborted();
    let count = state.db.retry_all_failed_jobs()?;
    state.worker.reset_circuit_breaker();
    let _ = app.emit("queue-updated", "all");
    Ok(count)
}

#[tauri::command]
pub async fn reorder_queue_job(
    state: State<'_, AppState>,
    app: AppHandle,
    job_id: String,
    new_created_at: i64,
) -> AppResult<()> {
    state.db.reorder_job_priority(&job_id, new_created_at)?;
    let _ = app.emit("queue-updated", &job_id);
    Ok(())
}
