use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const MIGRATION_001: &str = include_str!("../../migrations/001_initial.sql");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualityPreset {
    Best,
    Mp4_1080p,
    Mp4_720p,
    AudioOnly,
}

impl QualityPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            QualityPreset::Best => "best",
            QualityPreset::Mp4_1080p => "1080p",
            QualityPreset::Mp4_720p => "720p",
            QualityPreset::AudioOnly => "audio_only",
        }
    }

    pub fn from_str_lossy(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "1080p" | "mp4_1080p" => QualityPreset::Mp4_1080p,
            "720p" | "mp4_720p" => QualityPreset::Mp4_720p,
            "audio_only" | "audio" | "m4a" | "mp3" => QualityPreset::AudioOnly,
            _ => QualityPreset::Best,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFileRecord {
    pub id: String,
    pub job_id: String,
    pub slide_index: usize,
    pub media_type: String, // "image" | "video" | "audio"
    pub file_path: String,
    pub file_size_bytes: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub file_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub source_url: String,
    pub shortcode: Option<String>,
    pub url_type: String,
    pub author_username: Option<String>,
    pub caption: Option<String>,
    pub thumbnail_url: Option<String>,
    pub status: String, // "queued" | "running" | "paused" | "completed" | "failed" | "cancelled"
    pub selected_indices: Vec<usize>,
    pub quality_preset: String,
    pub output_dir: String,
    pub naming_template: String,
    pub save_caption: bool,
    pub save_metadata: bool,
    pub embed_thumbnail: bool,
    pub error_message: Option<String>,
    pub progress_percent: f64,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub media_files: Vec<MediaFileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewJobRequest {
    pub source_url: String,
    pub shortcode: Option<String>,
    pub url_type: String,
    pub author_username: Option<String>,
    pub caption: Option<String>,
    pub thumbnail_url: Option<String>,
    pub selected_indices: Vec<usize>,
    pub quality_preset: String,
    pub output_dir: Option<String>,
    pub naming_template: Option<String>,
    pub save_caption: Option<bool>,
    pub save_metadata: Option<bool>,
    pub embed_thumbnail: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryFilter {
    pub search_query: Option<String>,
    pub media_type_filter: Option<String>, // "all" | "reel" | "post" | "carousel" | "stories"
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub output_dir: String,
    pub naming_template: String,
    pub default_quality: String,
    pub concurrency: usize,
    pub jitter_min_ms: u64,
    pub jitter_max_ms: u64,
    pub proxy_url: Option<String>,
    pub clipboard_monitor: bool,
    pub auto_update_engines: bool,
    pub save_caption_by_default: bool,
    pub save_metadata_by_default: bool,
    pub embed_thumbnail_by_default: bool,
    pub auth_mode: String, // "anonymous" | "webview_vault" | "browser_cookies" | "cookie_file"
    pub selected_browser: Option<String>,
    pub connected_username: Option<String>,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let default_dl = dirs::download_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("InstaFlow");

        Self {
            output_dir: default_dl.to_string_lossy().to_string(),
            naming_template: "{username}/{date}_{shortcode}_{index}.{ext}".to_string(),
            default_quality: "best".to_string(),
            concurrency: 2,
            jitter_min_ms: 2000,
            jitter_max_ms: 4500,
            proxy_url: None,
            clipboard_monitor: true,
            auto_update_engines: true,
            save_caption_by_default: false,
            save_metadata_by_default: false,
            embed_thumbnail_by_default: true,
            auth_mode: "anonymous".to_string(),
            selected_browser: None,
            connected_username: None,
            theme: "dark".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Repository {
    conn: Arc<Mutex<Connection>>,
}

impl Repository {
    pub fn new(db_path: &Path) -> AppResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        repo.run_migrations()?;
        repo.recover_interrupted_jobs()?;
        Ok(repo)
    }

    pub fn new_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        let repo = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        repo.run_migrations()?;
        Ok(repo)
    }

    fn run_migrations(&self) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute_batch(MIGRATION_001)?;
        Ok(())
    }

    /// Resets any jobs left in `'running'` state during an unexpected exit back to `'queued'`.
    pub fn recover_interrupted_jobs(&self) -> AppResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let updated = conn.execute(
            "UPDATE jobs SET status = 'queued', error_message = NULL WHERE status = 'running'",
            [],
        )?;
        Ok(updated)
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let raw: Option<String> = conn
            .query_row(
                "SELECT value_json FROM settings WHERE key = 'app_settings'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(json_str) = raw {
            if let Ok(mut parsed) = serde_json::from_str::<AppSettings>(&json_str) {
                parsed.concurrency = parsed.concurrency.clamp(1, 3);
                return Ok(parsed);
            }
        }

        let default_settings = AppSettings::default();
        let json_str = serde_json::to_string(&default_settings)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value_json) VALUES ('app_settings', ?1)",
            params![json_str],
        )?;
        Ok(default_settings)
    }

    pub fn save_settings(&self, mut settings: AppSettings) -> AppResult<AppSettings> {
        settings.concurrency = settings.concurrency.clamp(1, 3);
        if settings.jitter_min_ms < 500 {
            settings.jitter_min_ms = 500;
        }
        if settings.jitter_max_ms < settings.jitter_min_ms {
            settings.jitter_max_ms = settings.jitter_min_ms + 1000;
        }
        let json_str =
            serde_json::to_string(&settings).map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value_json) VALUES ('app_settings', ?1)",
            params![json_str],
        )?;
        Ok(settings)
    }

    pub fn insert_job(&self, job: &JobRecord) -> AppResult<()> {
        let indices_json = serde_json::to_string(&job.selected_indices)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;

        conn.execute(
            "INSERT INTO jobs (
                id, source_url, shortcode, url_type, author_username, caption, thumbnail_url,
                status, selected_indices_json, quality_preset, output_dir, error_message,
                save_caption, save_metadata, embed_thumbnail, Naming_template,
                progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                job.id,
                job.source_url,
                job.shortcode,
                job.url_type,
                job.author_username,
                job.caption,
                job.thumbnail_url,
                job.status,
                indices_json,
                job.quality_preset,
                job.output_dir,
                job.error_message,
                if job.save_caption { 1 } else { 0 },
                if job.save_metadata { 1 } else { 0 },
                if job.embed_thumbnail { 1 } else { 0 },
                job.naming_template,
                job.progress_percent,
                job.downloaded_bytes as i64,
                job.total_bytes.map(|b| b as i64),
                job.created_at,
                job.completed_at,
            ],
        )?;
        Ok(())
    }

    pub fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
        completed_at: Option<i64>,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute(
            "UPDATE jobs SET status = ?1, error_message = ?2, completed_at = COALESCE(?3, completed_at) WHERE id = ?4",
            params![status, error_message, completed_at, job_id],
        )?;
        Ok(())
    }

    pub fn update_job_metadata(
        &self,
        job_id: &str,
        author_username: Option<&str>,
        caption: Option<&str>,
        thumbnail_url: Option<&str>,
        shortcode: Option<&str>,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute(
            "UPDATE jobs SET
                author_username = COALESCE(?1, author_username),
                caption = COALESCE(?2, caption),
                thumbnail_url = COALESCE(?3, thumbnail_url),
                shortcode = COALESCE(?4, shortcode)
             WHERE id = ?5",
            params![author_username, caption, thumbnail_url, shortcode, job_id],
        )?;
        Ok(())
    }

    pub fn update_job_progress(
        &self,
        job_id: &str,
        progress_percent: f64,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
    ) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute(
            "UPDATE jobs SET progress_percent = ?1, downloaded_bytes = ?2, total_bytes = COALESCE(?3, total_bytes) WHERE id = ?4",
            params![
                progress_percent,
                downloaded_bytes as i64,
                total_bytes.map(|b| b as i64),
                job_id
            ],
        )?;
        Ok(())
    }

    pub fn insert_media_file(&self, media: &MediaFileRecord) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute(
            "INSERT OR REPLACE INTO media_files (
                id, job_id, slide_index, media_type, file_path, file_size_bytes, width, height, duration_seconds
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                media.id,
                media.job_id,
                media.slide_index as i64,
                media.media_type,
                media.file_path,
                media.file_size_bytes.map(|b| b as i64),
                media.width,
                media.height,
                media.duration_seconds,
            ],
        )?;
        Ok(())
    }

    pub fn get_job(&self, job_id: &str) -> AppResult<Option<JobRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT id, source_url, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, Naming_template,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
             FROM jobs WHERE id = ?1",
        )?;

        let job_opt = stmt
            .query_row(params![job_id], |row| Self::map_job_row(row))
            .optional()?;

        if let Some(mut job) = job_opt {
            job.media_files = Self::fetch_media_files_for_job(&conn, &job.id)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    pub fn get_next_queued_job(&self) -> AppResult<Option<JobRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT id, source_url, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, Naming_template,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
             FROM jobs WHERE status = 'queued' ORDER BY created_at ASC LIMIT 1",
        )?;

        let job_opt = stmt
            .query_row([], |row| Self::map_job_row(row))
            .optional()?;

        if let Some(mut job) = job_opt {
            job.media_files = Self::fetch_media_files_for_job(&conn, &job.id)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    pub fn list_queue_jobs(&self) -> AppResult<Vec<JobRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT id, source_url, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, Naming_template,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
             FROM jobs
             ORDER BY
                CASE status
                    WHEN 'running' THEN 0
                    WHEN 'queued' THEN 1
                    WHEN 'paused' THEN 2
                    WHEN 'failed' THEN 3
                    WHEN 'completed' THEN 4
                    ELSE 5
                END ASC,
                created_at DESC
             LIMIT 300",
        )?;

        let rows = stmt.query_map([], |row| Self::map_job_row(row))?;
        let mut jobs = Vec::new();
        for r in rows {
            let mut job = r?;
            job.media_files = Self::fetch_media_files_for_job(&conn, &job.id)?;
            jobs.push(job);
        }
        Ok(jobs)
    }

    pub fn query_history(&self, filter: &HistoryFilter) -> AppResult<Vec<JobRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;

        let limit = filter.limit.unwrap_or(500) as i64;
        let offset = filter.offset.unwrap_or(0) as i64;
        let search = filter
            .search_query
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
        let media_filter = filter
            .media_type_filter
            .as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty() && s != "all");

        let mut stmt = conn.prepare(
            "SELECT id, source_url, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, Naming_template,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
             FROM jobs
             WHERE status = 'completed'
             ORDER BY completed_at DESC, created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let rows = stmt.query_map(params![limit, offset], |row| Self::map_job_row(row))?;
        let mut results = Vec::new();

        for r in rows {
            let mut job = r?;
            if let Some(ref q) = search {
                let matches_user = job
                    .author_username
                    .as_ref()
                    .map(|u| u.to_lowercase().contains(q))
                    .unwrap_or(false);
                let matches_caption = job
                    .caption
                    .as_ref()
                    .map(|c| c.to_lowercase().contains(q))
                    .unwrap_or(false);
                let matches_code = job
                    .shortcode
                    .as_ref()
                    .map(|s| s.to_lowercase().contains(q))
                    .unwrap_or(false);
                let matches_url = job.source_url.to_lowercase().contains(q);
                if !(matches_user || matches_caption || matches_code || matches_url) {
                    continue;
                }
            }

            if let Some(ref mf) = media_filter {
                let ut = job.url_type.to_lowercase();
                if mf == "reel" && ut != "reel" && ut != "tv" {
                    continue;
                } else if mf == "carousel" && ut != "carousel" && job.selected_indices.len() <= 1 {
                    continue;
                } else if mf == "post" && ut != "post" {
                    continue;
                } else if mf == "stories" && ut != "stories" && ut != "highlight" {
                    continue;
                }
            }

            if let Some(df) = filter.date_from {
                if job.created_at < df {
                    continue;
                }
            }
            if let Some(dt) = filter.date_to {
                if job.created_at > dt {
                    continue;
                }
            }

            job.media_files = Self::fetch_media_files_for_job(&conn, &job.id)?;
            results.push(job);
        }

        Ok(results)
    }

    pub fn pause_all_queued(&self) -> AppResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let count = conn.execute(
            "UPDATE jobs SET status = 'paused' WHERE status = 'queued'",
            [],
        )?;
        Ok(count)
    }

    pub fn resume_all_paused(&self) -> AppResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let count = conn.execute(
            "UPDATE jobs SET status = 'queued', error_message = NULL WHERE status = 'paused'",
            [],
        )?;
        Ok(count)
    }

    pub fn retry_failed_jobs(&self) -> AppResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let count = conn.execute(
            "UPDATE jobs SET status = 'queued', error_message = NULL, progress_percent = 0.0 WHERE status IN ('failed', 'cancelled')",
            [],
        )?;
        Ok(count)
    }

    pub fn clear_completed_jobs(&self) -> AppResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        let count = conn.execute("DELETE FROM jobs WHERE status = 'cancelled'", [])?;
        Ok(count)
    }

    pub fn delete_job(&self, job_id: &str, delete_disk_files: bool) -> AppResult<()> {
        if delete_disk_files {
            if let Some(job) = self.get_job(job_id)? {
                for mf in job.media_files {
                    let path = Path::new(&mf.file_path);
                    if path.exists() {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute("DELETE FROM jobs WHERE id = ?1", params![job_id])?;
        Ok(())
    }

    pub fn reorder_job(&self, job_id: &str, new_created_at: i64) -> AppResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Mutex poisoned: {}", e)))?;
        conn.execute(
            "UPDATE jobs SET created_at = ?1 WHERE id = ?2",
            params![new_created_at, job_id],
        )?;
        Ok(())
    }

    fn fetch_media_files_for_job(
        conn: &Connection,
        job_id: &str,
    ) -> Result<Vec<MediaFileRecord>, rusqlite::Error> {
        let mut stmt = conn.prepare(
            "SELECT id, job_id, slide_index, media_type, file_path, file_size_bytes, width, height, duration_seconds
             FROM media_files WHERE job_id = ?1 ORDER BY slide_index ASC",
        )?;
        let rows = stmt.query_map(params![job_id], |row| {
            let file_path: String = row.get(4)?;
            let exists = Path::new(&file_path).exists();
            Ok(MediaFileRecord {
                id: row.get(0)?,
                job_id: row.get(1)?,
                slide_index: row.get::<_, i64>(2)? as usize,
                media_type: row.get(3)?,
                file_path,
                file_size_bytes: row.get::<_, Option<i64>>(5)?.map(|b| b as u64),
                width: row.get(6)?,
                height: row.get(7)?,
                duration_seconds: row.get(8)?,
                file_exists: exists,
            })
        })?;
        let mut files = Vec::new();
        for r in rows {
            files.push(r?);
        }
        Ok(files)
    }

    fn map_job_row(row: &rusqlite::Row<'_>) -> Result<JobRecord, rusqlite::Error> {
        let indices_raw: Option<String> = row.get(8)?;
        let selected_indices: Vec<usize> = indices_raw
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let naming_template: Option<String> = row.get(15)?;

        Ok(JobRecord {
            id: row.get(0)?,
            source_url: row.get(1)?,
            shortcode: row.get(2)?,
            url_type: row.get(3)?,
            author_username: row.get(4)?,
            caption: row.get(5)?,
            thumbnail_url: row.get(6)?,
            status: row.get(7)?,
            selected_indices,
            quality_preset: row.get(9)?,
            output_dir: row.get(10)?,
            error_message: row.get(11)?,
            save_caption: row.get::<_, i64>(12)? != 0,
            save_metadata: row.get::<_, i64>(13)? != 0,
            embed_thumbnail: row.get::<_, i64>(14)? != 0,
            naming_template: naming_template
                .unwrap_or_else(|| "{username}/{date}_{shortcode}_{index}.{ext}".to_string()),
            progress_percent: row.get(16)?,
            downloaded_bytes: row.get::<_, i64>(17)? as u64,
            total_bytes: row.get::<_, Option<i64>>(18)?.map(|b| b as u64),
            created_at: row.get(19)?,
            completed_at: row.get(20)?,
            media_files: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_migrations_and_job_lifecycle() {
        let repo = Repository::new_in_memory().expect("in-memory db init");
        let settings = repo.get_settings().expect("default settings");
        assert_eq!(settings.concurrency, 2);

        let job = JobRecord {
            id: "job-101".to_string(),
            source_url: "https://www.instagram.com/reel/C9xYz123/".to_string(),
            shortcode: Some("C9xYz123".to_string()),
            url_type: "reel".to_string(),
            author_username: Some("natgeo".to_string()),
            caption: Some("Amazing wildlife reel".to_string()),
            thumbnail_url: None,
            status: "running".to_string(),
            selected_indices: vec![1],
            quality_preset: "best".to_string(),
            output_dir: "C:\\Downloads\\InstaFlow".to_string(),
            naming_template: "{username}/{date}_{shortcode}_{index}.{ext}".to_string(),
            save_caption: true,
            save_metadata: false,
            embed_thumbnail: true,
            error_message: None,
            progress_percent: 42.5,
            downloaded_bytes: 1024,
            total_bytes: Some(2048),
            created_at: 1730000000,
            completed_at: None,
            media_files: vec![],
        };

        repo.insert_job(&job).expect("insert job");

        // Verify crash recovery resets running -> queued
        let recovered = repo.recover_interrupted_jobs().expect("recover");
        assert_eq!(recovered, 1);

        let next = repo.get_next_queued_job().expect("get next").expect("exists");
        assert_eq!(next.id, "job-101");
        assert_eq!(next.status, "queued");

        // Insert media file and complete job
        repo.insert_media_file(&MediaFileRecord {
            id: "mf-1".to_string(),
            job_id: "job-101".to_string(),
            slide_index: 1,
            media_type: "video".to_string(),
            file_path: "C:\\Downloads\\InstaFlow\\natgeo\\2026-11-02_C9xYz123_01.mp4".to_string(),
            file_size_bytes: Some(2048),
            width: Some(1080),
            height: Some(1920),
            duration_seconds: Some(15.4),
            file_exists: false,
        })
        .expect("insert media file");

        repo.update_job_status("job-101", "completed", None, Some(1730000050))
            .expect("complete job");

        let history = repo
            .query_history(&HistoryFilter {
                search_query: Some("natgeo".to_string()),
                ..Default::default()
            })
            .expect("query history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].media_files.len(), 1);
    }
}
