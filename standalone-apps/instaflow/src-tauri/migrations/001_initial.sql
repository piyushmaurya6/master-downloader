-- InstaFlow Initial SQLite Schema (001_initial.sql)
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY NOT NULL,
    source_url TEXT NOT NULL,
    shortcode TEXT,
    url_type TEXT NOT NULL,
    author_username TEXT,
    caption TEXT,
    thumbnail_url TEXT,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'paused', 'completed', 'failed', 'cancelled')),
    selected_indices_json TEXT,
    quality_preset TEXT NOT NULL,
    output_dir TEXT NOT NULL,
    error_message TEXT,
    save_caption INTEGER NOT NULL DEFAULT 0,
    save_metadata INTEGER NOT NULL DEFAULT 0,
    embed_thumbnail INTEGER NOT NULL DEFAULT 1,
    Naming_template TEXT,
    progress_percent REAL NOT NULL DEFAULT 0.0,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER,
    created_at INTEGER NOT NULL,
    completed_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_author_username ON jobs(author_username);
CREATE INDEX IF NOT EXISTS idx_jobs_shortcode ON jobs(shortcode);

CREATE TABLE IF NOT EXISTS media_files (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    slide_index INTEGER NOT NULL,
    media_type TEXT NOT NULL CHECK (media_type IN ('image', 'video', 'audio')),
    file_path TEXT NOT NULL,
    file_size_bytes INTEGER,
    width INTEGER,
    height INTEGER,
    duration_seconds REAL
);

CREATE INDEX IF NOT EXISTS idx_media_files_job_id ON media_files(job_id);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL
);
