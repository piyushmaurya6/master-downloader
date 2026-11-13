use crate::models::{DownloadItem, HistoryItem};
use crate::utils::app_data_dir;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

const MIGRATION_V1_SQL: &str = include_str!("../../migrations/V1__initial_schema.sql");
const MIGRATION_V2_SQL: &str = include_str!("../../migrations/V2__indices_and_metadata.sql");

struct MigrationStep {
    version: u32,
    description: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[MigrationStep] = &[
    MigrationStep {
        version: 1,
        description: "Initial download_history and download_queue schema (1.0.0)",
        sql: MIGRATION_V1_SQL,
    },
    MigrationStep {
        version: 2,
        description: "Add performance indices and app_metadata table (1.1.0+ forward compatibility)",
        sql: MIGRATION_V2_SQL,
    },
];

#[derive(Clone)]
pub struct HistoryStore {
    db_path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl HistoryStore {
    pub fn new() -> Result<Self, String> {
        let db_path = app_data_dir().join("flowdl.db");
        Self::with_path(db_path)
    }

    pub fn with_path(db_path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let store = Self {
            db_path,
            lock: Arc::new(Mutex::new(())),
        };
        store.run_migrations()?;
        Ok(store)
    }

    fn connect(&self) -> Result<Connection, String> {
        Connection::open(&self.db_path).map_err(|e| format!("SQLite connection error: {}", e))
    }

    /// Runs versioned, transactional SQLite migrations safely.
    /// Interrupted migrations automatically roll back; existing download history and queue rows are preserved.
    pub fn run_migrations(&self) -> Result<u32, String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let mut conn = self.connect()?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("Failed to initialize schema_migrations table: {}", e))?;

        let current_ver: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let mut latest = current_ver;
        for step in MIGRATIONS {
            if step.version > current_ver {
                let tx = conn
                    .transaction()
                    .map_err(|e| format!("Failed to start migration v{} transaction: {}", step.version, e))?;
                tx.execute_batch(step.sql)
                    .map_err(|e| format!("Migration v{} failed: {}", step.version, e))?;
                tx.execute(
                    "INSERT INTO schema_migrations (version, description, applied_at) VALUES (?1, ?2, ?3)",
                    params![step.version, step.description, Utc::now().to_rfc3339()],
                )
                .map_err(|e| format!("Failed to record migration v{}: {}", step.version, e))?;
                tx.pragma_update(None, "user_version", step.version)
                    .map_err(|e| format!("Failed to update user_version pragma: {}", e))?;
                tx.commit()
                    .map_err(|e| format!("Failed to commit migration v{}: {}", step.version, e))?;
                latest = step.version;
            }
        }

        Ok(latest)
    }

    pub fn get_schema_version(&self) -> Result<u32, String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let conn = self.connect()?;
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to read schema version: {}", e))
    }

    pub fn add_history(&self, item: &HistoryItem) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let conn = self.connect()?;
        let req_json = serde_json::to_string(&item.request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO download_history (
                id, title, thumbnail, uploader, source_url, extractor,
                downloaded_at, mode, format_summary, quality, file_size,
                output_path, output_dir, status, error_message, request_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                item.id,
                item.title,
                item.thumbnail,
                item.uploader,
                item.source_url,
                item.extractor,
                item.downloaded_at,
                item.mode,
                item.format_summary,
                item.quality,
                item.file_size.map(|n| n as i64),
                item.output_path,
                item.output_dir,
                item.status,
                item.error_message,
                req_json
            ],
        )
        .map_err(|e| format!("Failed to insert history item: {}", e))?;
        Ok(())
    }

    pub fn list_history(&self) -> Result<Vec<HistoryItem>, String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare(
                "SELECT id, title, thumbnail, uploader, source_url, extractor,
                        downloaded_at, mode, format_summary, quality, file_size,
                        output_path, output_dir, status, error_message, request_json
                 FROM download_history
                 ORDER BY downloaded_at DESC",
            )
            .map_err(|e| format!("Failed to query history: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                let output_path: Option<String> = row.get(11)?;
                let status: String = row.get(13)?;
                let file_exists = match &output_path {
                    Some(p) if !p.is_empty() && status == "completed" => Path::new(p).exists(),
                    _ => false,
                };
                let file_size_raw: Option<i64> = row.get(10)?;
                let req_json: String = row.get(15)?;
                let request = serde_json::from_str(&req_json).map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        15,
                        "request_json".to_string(),
                        rusqlite::types::Type::Text,
                    )
                })?;

                Ok(HistoryItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    thumbnail: row.get(2)?,
                    uploader: row.get(3)?,
                    source_url: row.get(4)?,
                    extractor: row.get(5)?,
                    downloaded_at: row.get(6)?,
                    mode: row.get(7)?,
                    format_summary: row.get(8)?,
                    quality: row.get(9)?,
                    file_size: file_size_raw.map(|n| n as u64),
                    output_path,
                    output_dir: row.get(12)?,
                    status,
                    file_exists,
                    error_message: row.get(14)?,
                    request,
                })
            })
            .map_err(|e| format!("Failed to map history rows: {}", e))?;

        let mut items = Vec::new();
        for row in rows {
            if let Ok(item) = row {
                items.push(item);
            }
        }
        Ok(items)
    }

    /// Removes the history entry from SQLite WITHOUT deleting the downloaded file on disk.
    pub fn remove_history_entry(&self, id: &str) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let conn = self.connect()?;
        conn.execute("DELETE FROM download_history WHERE id = ?1", params![id])
            .map_err(|e| format!("Failed to delete history entry: {}", e))?;
        Ok(())
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let conn = self.connect()?;
        conn.execute("DELETE FROM download_history", [])
            .map_err(|e| format!("Failed to clear history: {}", e))?;
        Ok(())
    }

    pub fn save_queue(&self, items: &[DownloadItem]) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let mut conn = self.connect()?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to begin queue transaction: {}", e))?;
        tx.execute("DELETE FROM download_queue", [])
            .map_err(|e| format!("Failed to clear queue table: {}", e))?;

        for (idx, item) in items.iter().enumerate() {
            let json = serde_json::to_string(item)
                .map_err(|e| format!("Failed to serialize queue item: {}", e))?;
            tx.execute(
                "INSERT INTO download_queue (id, position, item_json) VALUES (?1, ?2, ?3)",
                params![item.id, idx as i64, json],
            )
            .map_err(|e| format!("Failed to save queue item: {}", e))?;
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit queue transaction: {}", e))?;
        Ok(())
    }

    /// Loads persisted queue items on startup. Any item that was mid-download when the app closed
    /// is safely restored as "queued" (never falsely marked as completed).
    pub fn load_queue(&self) -> Result<Vec<DownloadItem>, String> {
        let _guard = self.lock.lock().map_err(|_| "Mutex poisoned")?;
        let conn = self.connect()?;
        let mut stmt = conn
            .prepare("SELECT item_json FROM download_queue ORDER BY position ASC")
            .map_err(|e| format!("Failed to prepare queue load query: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })
            .map_err(|e| format!("Failed to read queue rows: {}", e))?;

        let mut restored = Vec::new();
        for r in rows {
            if let Ok(json) = r {
                if let Ok(mut item) = serde_json::from_str::<DownloadItem>(&json) {
                    if item.status == "downloading"
                        || item.status == "analyzing"
                        || item.status == "postprocessing"
                        || item.status == "retrying"
                    {
                        item.status = "queued".to_string();
                        item.speed_bytes_per_sec = None;
                        item.eta_seconds = None;
                        item.postprocess_stage = None;
                    }
                    restored.push(item);
                }
            }
        }
        Ok(restored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DownloadRequest;

    fn sample_request() -> DownloadRequest {
        DownloadRequest {
            url: "https://www.youtube.com/watch?v=test123".to_string(),
            title: "Unicode & Spaces Video 🎵 测试".to_string(),
            thumbnail: None,
            uploader: Some("FlowDL Channel".to_string()),
            duration: Some(90.0),
            extractor: "Youtube".to_string(),
            mode: "video".to_string(),
            video_quality: "1080".to_string(),
            exact_resolution: false,
            container: "mp4".to_string(),
            video_codec: "auto".to_string(),
            preferred_fps: None,
            audio_format: "mp3".to_string(),
            audio_quality: "320".to_string(),
            download_subtitles: false,
            subtitle_lang: "en".to_string(),
            embed_thumbnail: true,
            embed_metadata: true,
            download_chapters: false,
            normalize_audio: false,
            preserve_original_filename: false,
            filename_template: None,
            speed_limit_kbps: None,
            playlist_mode: "single".to_string(),
            output_dir: None,
        }
    }

    #[test]
    fn test_missing_output_file_in_history_and_safe_removal() {
        let temp_db = std::env::temp_dir().join(format!("flowdl_test_{}.db", uuid::Uuid::new_v4()));
        let store = HistoryStore::with_path(temp_db.clone()).unwrap();

        let item = HistoryItem {
            id: "hist-1".to_string(),
            title: "Unicode & Spaces Video 🎵 测试".to_string(),
            thumbnail: None,
            uploader: Some("FlowDL".to_string()),
            source_url: "https://www.youtube.com/watch?v=test123".to_string(),
            extractor: "Youtube".to_string(),
            downloaded_at: "2026-10-28T12:00:00Z".to_string(),
            mode: "video".to_string(),
            format_summary: "1080p · MP4".to_string(),
            quality: "1080p".to_string(),
            file_size: Some(1048576),
            output_path: Some("C:\\NonExistentPath\\Unicode & Spaces 🎵.mp4".to_string()),
            output_dir: "C:\\NonExistentPath".to_string(),
            status: "completed".to_string(),
            file_exists: true,
            error_message: None,
            request: sample_request(),
        };

        store.add_history(&item).unwrap();
        let list = store.list_history().unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list[0].file_exists, "Missing file on disk must report file_exists = false");

        store.remove_history_entry("hist-1").unwrap();
        assert_eq!(store.list_history().unwrap().len(), 0);
        let _ = std::fs::remove_file(temp_db);
    }

    #[test]
    fn test_queue_recovery_on_restart_does_not_mark_interrupted_as_completed() {
        let temp_db = std::env::temp_dir().join(format!("flowdl_q_test_{}.db", uuid::Uuid::new_v4()));
        let store = HistoryStore::with_path(temp_db.clone()).unwrap();

        let active_item = DownloadItem {
            id: "dl-1".to_string(),
            url: "https://www.youtube.com/watch?v=test123".to_string(),
            title: "Interrupted Download".to_string(),
            thumbnail: None,
            uploader: None,
            duration: Some(60.0),
            extractor: "Youtube".to_string(),
            mode: "video".to_string(),
            format_summary: "1080p · MP4".to_string(),
            quality_label: "1080p".to_string(),
            status: "downloading".to_string(),
            postprocess_stage: Some("downloading_video".to_string()),
            progress: 48.5,
            downloaded_bytes: 500000,
            total_bytes: Some(1000000),
            speed_bytes_per_sec: Some(102400.0),
            eta_seconds: Some(5),
            output_path: None,
            output_dir: "C:\\Downloads".to_string(),
            error_message: None,
            technical_error: None,
            error_category: None,
            recovery_action: None,
            suggest_engine_update: false,
            retry_count: 0,
            max_retries: 3,
            created_at: "2026-10-28T12:00:00Z".to_string(),
            completed_at: None,
            request: sample_request(),
        };

        store.save_queue(&[active_item]).unwrap();
        let restored = store.load_queue().unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].status, "queued");
        assert_ne!(restored[0].status, "completed");
        let _ = std::fs::remove_file(temp_db);
    }

    #[test]
    fn test_schema_migrations_preserve_legacy_history_and_queue_across_versions() {
        let temp_db =
            std::env::temp_dir().join(format!("flowdl_mig_test_{}.db", uuid::Uuid::new_v4()));
        {
            // Simulate a v1.0.0 pre-migration database with existing history rows
            let conn = Connection::open(&temp_db).unwrap();
            conn.execute_batch(MIGRATION_V1_SQL).unwrap();
            let req_json = serde_json::to_string(&sample_request()).unwrap();
            conn.execute(
                "INSERT INTO download_history (
                    id, title, thumbnail, uploader, source_url, extractor,
                    downloaded_at, mode, format_summary, quality, file_size,
                    output_path, output_dir, status, error_message, request_json
                ) VALUES ('legacy-1', 'Legacy Video', NULL, 'Creator', 'https://youtube.com/watch?v=1', 'Youtube',
                          '2026-01-01T00:00:00Z', 'video', '1080p · MP4', '1080p', 2048,
                          NULL, 'C:\\Downloads', 'completed', NULL, ?1)",
                params![req_json],
            )
            .unwrap();
        }

        // Open with HistoryStore, which runs V1 and V2 migrations
        let store = HistoryStore::with_path(temp_db.clone()).unwrap();
        assert_eq!(store.get_schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        let history = store.list_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "legacy-1");
        assert_eq!(history[0].title, "Legacy Video");

        let _ = std::fs::remove_file(temp_db);
    }
}
