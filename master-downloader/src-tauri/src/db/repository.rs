use crate::error::{AppError, AppResult};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

const MIGRATION_001: &str = include_str!("../../migrations/001_initial.sql");

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdvancedJobOptions {
    pub container: Option<String>,          // "auto" | "mp4" | "mkv" | "webm"
    pub preferred_codec: Option<String>,    // "auto" | "h264" | "h265" | "vp9" | "av1"
    pub preferred_fps: Option<u32>,         // 60 | 30
    pub exact_resolution: bool,
    pub audio_format: Option<String>,       // "mp3" | "m4a" | "flac" | "wav" | "opus"
    pub audio_bitrate: Option<String>,      // "320" | "256" | "192" | "128"
    pub audio_normalization: bool,          // EBU R128 loudnorm
    pub write_subtitles: bool,
    pub subtitle_langs: Option<String>,     // e.g. "en,.*-orig"
    pub embed_chapters: bool,
    pub playlist_mode: Option<String>,      // "single" | "playlist"
    pub speed_limit_kbps: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFileRecord {
    pub id: String,
    pub job_id: String,
    pub slide_index: usize,
    pub media_type: String,
    pub file_path: String,
    pub file_size_bytes: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub source_url: String,
    pub platform: String,
    pub shortcode: Option<String>,
    pub url_type: String,
    pub author_username: Option<String>,
    pub caption: Option<String>,
    pub thumbnail_url: Option<String>,
    pub status: String,
    pub selected_indices: Vec<usize>,
    pub quality_preset: String,
    pub output_dir: String,
    pub error_message: Option<String>,
    pub save_caption: bool,
    pub save_metadata: bool,
    pub embed_thumbnail: bool,
    pub naming_template: Option<String>,
    pub options: AdvancedJobOptions,
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
    pub platform: Option<String>,
    pub shortcode: Option<String>,
    pub url_type: String,
    pub author_username: Option<String>,
    pub caption: Option<String>,
    pub thumbnail_url: Option<String>,
    pub selected_indices: Vec<usize>,
    pub quality_preset: String,
    pub output_dir: Option<String>,
    pub save_caption: bool,
    pub save_metadata: bool,
    pub embed_thumbnail: bool,
    pub naming_template: Option<String>,
    #[serde(default)]
    pub options: AdvancedJobOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryFilter {
    pub search_query: Option<String>,
    pub platform: Option<String>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub output_dir: String,
    pub default_quality: String,
    pub default_container: String,
    pub default_audio_format: String,
    pub default_audio_bitrate: String,
    pub audio_normalization: bool,
    pub concurrency: usize,
    pub jitter_min_ms: u64,
    pub jitter_max_ms: u64,
    pub speed_limit_kbps: Option<u64>,
    pub clipboard_monitor: bool,
    pub auto_download_on_detect: bool,
    pub organize_by_platform: bool,
    pub organize_by_uploader: bool,
    pub embed_thumbnail: bool,
    pub save_caption_sidecar: bool,
    pub save_metadata_json: bool,
    pub write_subtitles: bool,
    pub embed_chapters: bool,
    pub naming_template: String,
    pub browser_cookie_source: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let default_dl = dirs::download_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("C:\\Downloads"))
            .join("MasterDownloader")
            .to_string_lossy()
            .to_string();

        Self {
            output_dir: default_dl,
            default_quality: "best".into(),
            default_container: "mp4".into(),
            default_audio_format: "mp3".into(),
            default_audio_bitrate: "320".into(),
            audio_normalization: false,
            concurrency: 3,
            jitter_min_ms: 250,
            jitter_max_ms: 600,
            speed_limit_kbps: None,
            clipboard_monitor: true,
            auto_download_on_detect: true,
            organize_by_platform: true,
            organize_by_uploader: true,
            embed_thumbnail: true,
            save_caption_sidecar: false,
            save_metadata_json: false,
            write_subtitles: false,
            embed_chapters: true,
            naming_template: "{username}_{date}_{shortcode}_{index}".into(),
            browser_cookie_source: None,
        }
    }
}

#[derive(Clone)]
pub struct DbRepository {
    conn: Arc<Mutex<Connection>>,
}

impl DbRepository {
    pub fn open_and_migrate(db_path: &Path) -> AppResult<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = match Self::init_sqlite_file(db_path) {
            Ok(c) => c,
            Err(first_err) => {
                tracing::warn!(
                    "Primary SQLite open/migration failed ({}); backing up corrupted DB and recreating",
                    first_err
                );
                let backup = db_path.with_extension(format!(
                    "corrupt.{}.db",
                    chrono::Utc::now().timestamp()
                ));
                let _ = std::fs::rename(db_path, &backup);
                let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
                let _ = std::fs::remove_file(db_path.with_extension("db-shm"));
                Self::init_sqlite_file(db_path)?
            }
        };

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_sqlite_file(db_path: &Path) -> AppResult<Connection> {
        let conn = Connection::open(db_path)?;
        let _ = conn.busy_timeout(std::time::Duration::from_millis(8000));
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = ON;")?;
        conn.execute_batch(MIGRATION_001)?;

        // Repair any YouTube watch?v= URLs that were previously misclassified as playlist due to &list=RD...
        let _ = conn.execute(
            "UPDATE jobs SET url_type = 'video', options_json = REPLACE(options_json, '\"playlist_mode\":\"playlist\"', '\"playlist_mode\":\"single\"') WHERE platform = 'youtube' AND source_url LIKE '%watch?v=%' AND url_type = 'playlist'",
            [],
        );

        // Ensure recovery of interrupted jobs on startup
        conn.execute(
            "UPDATE jobs SET status = 'queued' WHERE status = 'running'",
            [],
        )?;

        Ok(conn)
    }

    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        let _ = conn.busy_timeout(std::time::Duration::from_millis(5000));
        conn.execute_batch(MIGRATION_001)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn insert_job(&self, req: &NewJobRequest, default_output_dir: &str) -> AppResult<JobRecord> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let indices_json = serde_json::to_string(&req.selected_indices)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let options_json =
            serde_json::to_string(&req.options).map_err(|e| AppError::Internal(e.to_string()))?;
        let output_dir = req
            .output_dir
            .clone()
            .unwrap_or_else(|| default_output_dir.to_string());
        let platform = req.platform.clone().unwrap_or_else(|| "generic".into());

        {
            let guard = self
                .conn
                .lock()
                .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
            guard.execute(
                "INSERT INTO jobs (
                    id, source_url, platform, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, naming_template, options_json,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'queued', ?9, ?10, ?11, NULL, ?12, ?13, ?14, ?15, ?16, 0.0, 0, NULL, ?17, NULL)",
                params![
                    id,
                    req.source_url,
                    platform,
                    req.shortcode,
                    req.url_type,
                    req.author_username,
                    req.caption,
                    req.thumbnail_url,
                    indices_json,
                    req.quality_preset,
                    output_dir,
                    req.save_caption as i32,
                    req.save_metadata as i32,
                    req.embed_thumbnail as i32,
                    req.naming_template,
                    options_json,
                    now
                ],
            )?;
        }

        self.get_job(&id)?
            .ok_or_else(|| AppError::Database("Inserted job could not be retrieved.".into()))
    }

    pub fn get_job(&self, job_id: &str) -> AppResult<Option<JobRecord>> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;

        let mut stmt = guard.prepare(
            "SELECT id, source_url, platform, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, naming_template, options_json,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
             FROM jobs WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![job_id])?;
        if let Some(row) = rows.next()? {
            let mut job = map_job_row(row)?;
            job.media_files = Self::load_media_files(&guard, &job.id)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    pub fn claim_next_queued_job(&self) -> AppResult<Option<JobRecord>> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;

        let mut stmt = guard.prepare(
            "SELECT id FROM jobs WHERE status = 'queued' ORDER BY created_at ASC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let job_id: String = row.get(0)?;
        drop(rows);
        drop(stmt);

        guard.execute(
            "UPDATE jobs SET status = 'running', error_message = NULL WHERE id = ?1",
            params![job_id],
        )?;
        drop(guard);

        self.get_job(&job_id)
    }

    pub fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> AppResult<()> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        let completed_at = if status == "completed" {
            Some(chrono::Utc::now().timestamp())
        } else {
            None
        };
        guard.execute(
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
    ) -> AppResult<()> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        guard.execute(
            "UPDATE jobs SET
                author_username = COALESCE(?1, author_username),
                caption = COALESCE(?2, caption),
                thumbnail_url = COALESCE(?3, thumbnail_url)
             WHERE id = ?4",
            params![author_username, caption, thumbnail_url, job_id],
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
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        guard.execute(
            "UPDATE jobs SET progress_percent = ?1, downloaded_bytes = ?2, total_bytes = COALESCE(?3, total_bytes) WHERE id = ?4",
            params![
                progress_percent,
                downloaded_bytes as i64,
                total_bytes.map(|v| v as i64),
                job_id
            ],
        )?;
        Ok(())
    }

    pub fn insert_media_file(&self, file: &MediaFileRecord) -> AppResult<()> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        guard.execute(
            "INSERT INTO media_files (id, job_id, slide_index, media_type, file_path, file_size_bytes, width, height, duration_seconds)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                file.id,
                file.job_id,
                file.slide_index as i64,
                file.media_type,
                file.file_path,
                file.file_size_bytes.map(|b| b as i64),
                file.width,
                file.height,
                file.duration_seconds
            ],
        )?;
        Ok(())
    }

    pub fn list_active_queue(&self) -> AppResult<Vec<JobRecord>> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;

        let mut stmt = guard.prepare(
            "SELECT id, source_url, platform, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, naming_template, options_json,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
             FROM jobs
             WHERE status IN ('running', 'queued', 'paused', 'failed')
             ORDER BY CASE status
                WHEN 'running' THEN 0
                WHEN 'queued' THEN 1
                WHEN 'paused' THEN 2
                ELSE 3
             END, created_at ASC",
        )?;

        let mut rows = stmt.query([])?;
        let mut jobs = Vec::new();
        while let Some(row) = rows.next()? {
            let mut job = map_job_row(row)?;
            job.media_files = Self::load_media_files(&guard, &job.id)?;
            jobs.push(job);
        }
        Ok(jobs)
    }

    pub fn query_history(&self, filter: &HistoryFilter) -> AppResult<Vec<JobRecord>> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;

        let limit = filter.limit.unwrap_or(100) as i64;
        let offset = filter.offset.unwrap_or(0) as i64;
        let search = filter
            .search_query
            .as_ref()
            .map(|s| format!("%{}%", s.trim()))
            .unwrap_or_else(|| "%".to_string());

        let mut stmt = guard.prepare(
            "SELECT id, source_url, platform, shortcode, url_type, author_username, caption, thumbnail_url,
                    status, selected_indices_json, quality_preset, output_dir, error_message,
                    save_caption, save_metadata, embed_thumbnail, naming_template, options_json,
                    progress_percent, downloaded_bytes, total_bytes, created_at, completed_at
             FROM jobs
             WHERE (COALESCE(author_username, '') LIKE ?1
                    OR COALESCE(caption, '') LIKE ?1
                    OR COALESCE(shortcode, '') LIKE ?1
                    OR source_url LIKE ?1)
               AND (?2 IS NULL OR status = ?2)
               AND (?3 IS NULL OR platform = ?3)
             ORDER BY created_at DESC
             LIMIT ?4 OFFSET ?5",
        )?;

        let mut rows = stmt.query(params![
            search,
            filter.status,
            filter.platform,
            limit,
            offset
        ])?;
        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            let mut job = map_job_row(row)?;
            job.media_files = Self::load_media_files(&guard, &job.id)?;

            if let Some(ref mtype) = filter.media_type {
                if !mtype.is_empty() && mtype != "all" {
                    let matches = job.url_type.eq_ignore_ascii_case(mtype)
                        || job
                            .media_files
                            .iter()
                            .any(|f| f.media_type.eq_ignore_ascii_case(mtype));
                    if !matches {
                        continue;
                    }
                }
            }
            results.push(job);
        }
        Ok(results)
    }

    pub fn pause_all_active_jobs(&self) -> AppResult<usize> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        let count = guard.execute(
            "UPDATE jobs SET status = 'paused' WHERE status IN ('queued', 'running')",
            [],
        )?;
        Ok(count)
    }

    pub fn resume_all_paused_jobs(&self) -> AppResult<usize> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        let count = guard.execute(
            "UPDATE jobs SET status = 'queued', error_message = NULL WHERE status = 'paused'",
            [],
        )?;
        Ok(count)
    }

    pub fn retry_all_failed_jobs(&self) -> AppResult<usize> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        let count = guard.execute(
            "UPDATE jobs SET status = 'queued', error_message = NULL, progress_percent = 0.0 WHERE status IN ('failed', 'cancelled')",
            [],
        )?;
        Ok(count)
    }

    pub fn delete_job(&self, job_id: &str) -> AppResult<Vec<String>> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        let files = Self::load_media_files(&guard, job_id)?;
        let paths = files.into_iter().map(|f| f.file_path).collect();
        guard.execute("DELETE FROM jobs WHERE id = ?1", params![job_id])?;
        Ok(paths)
    }

    pub fn reorder_job_priority(&self, job_id: &str, new_timestamp: i64) -> AppResult<()> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        guard.execute(
            "UPDATE jobs SET created_at = ?1 WHERE id = ?2",
            params![new_timestamp, job_id],
        )?;
        Ok(())
    }

    pub fn get_settings(&self) -> AppResult<AppSettings> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        let mut stmt =
            guard.prepare("SELECT value_json FROM settings WHERE key = 'app_settings'")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let json_str: String = row.get(0)?;
            if let Ok(parsed) = serde_json::from_str::<AppSettings>(&json_str) {
                return Ok(parsed);
            }
        }
        Ok(AppSettings::default())
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        let guard = self
            .conn
            .lock()
            .map_err(|_| AppError::Database("DB lock poisoned".into()))?;
        let json_str = serde_json::to_string(settings)
            .map_err(|e| AppError::Internal(format!("Serialize settings error: {}", e)))?;
        guard.execute(
            "INSERT INTO settings (key, value_json) VALUES ('app_settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
            params![json_str],
        )?;
        Ok(())
    }

    fn load_media_files(conn: &Connection, job_id: &str) -> AppResult<Vec<MediaFileRecord>> {
        let mut stmt = conn.prepare(
            "SELECT id, job_id, slide_index, media_type, file_path, file_size_bytes, width, height, duration_seconds
             FROM media_files WHERE job_id = ?1 ORDER BY slide_index ASC",
        )?;
        let mut rows = stmt.query(params![job_id])?;
        let mut files = Vec::new();
        while let Some(row) = rows.next()? {
            files.push(MediaFileRecord {
                id: row.get(0)?,
                job_id: row.get(1)?,
                slide_index: row.get::<_, i64>(2)? as usize,
                media_type: row.get(3)?,
                file_path: row.get(4)?,
                file_size_bytes: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                width: row.get(6)?,
                height: row.get(7)?,
                duration_seconds: row.get(8)?,
            });
        }
        Ok(files)
    }
}

fn map_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRecord> {
    let indices_raw: Option<String> = row.get(9)?;
    let selected_indices: Vec<usize> = indices_raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let options_raw: Option<String> = row.get(17)?;
    let options: AdvancedJobOptions = options_raw
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    Ok(JobRecord {
        id: row.get(0)?,
        source_url: row.get(1)?,
        platform: row
            .get::<_, Option<String>>(2)?
            .unwrap_or_else(|| "generic".into()),
        shortcode: row.get(3)?,
        url_type: row.get(4)?,
        author_username: row.get(5)?,
        caption: row.get(6)?,
        thumbnail_url: row.get(7)?,
        status: row.get(8)?,
        selected_indices,
        quality_preset: row.get(10)?,
        output_dir: row.get(11)?,
        error_message: row.get(12)?,
        save_caption: row.get::<_, i32>(13)? != 0,
        save_metadata: row.get::<_, i32>(14)? != 0,
        embed_thumbnail: row.get::<_, i32>(15)? != 0,
        naming_template: row.get(16)?,
        options,
        progress_percent: row.get(18)?,
        downloaded_bytes: row.get::<_, i64>(19)? as u64,
        total_bytes: row.get::<_, Option<i64>>(20)?.map(|v| v as u64),
        created_at: row.get(21)?,
        completed_at: row.get(22)?,
        media_files: Vec::new(),
    })
}
