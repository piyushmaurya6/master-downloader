-- FlowDL SQLite Schema Migration V1 (App v1.0.0)
CREATE TABLE IF NOT EXISTS download_history (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    thumbnail TEXT,
    uploader TEXT,
    source_url TEXT NOT NULL,
    extractor TEXT NOT NULL,
    downloaded_at TEXT NOT NULL,
    mode TEXT NOT NULL,
    format_summary TEXT NOT NULL,
    quality TEXT NOT NULL,
    file_size INTEGER,
    output_path TEXT,
    output_dir TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT,
    request_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS download_queue (
    id TEXT PRIMARY KEY,
    position INTEGER NOT NULL,
    item_json TEXT NOT NULL
);
