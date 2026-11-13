-- FlowDL SQLite Schema Migration V2 (Forward-Compatible Indices & Metadata Table)
CREATE INDEX IF NOT EXISTS idx_history_downloaded_at ON download_history(downloaded_at DESC);
CREATE INDEX IF NOT EXISTS idx_history_status ON download_history(status);
CREATE INDEX IF NOT EXISTS idx_queue_position ON download_queue(position ASC);

CREATE TABLE IF NOT EXISTS app_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
