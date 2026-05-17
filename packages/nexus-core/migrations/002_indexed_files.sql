CREATE TABLE IF NOT EXISTS indexed_files (
    workspace_id    TEXT NOT NULL,
    file_path       TEXT NOT NULL,
    content_hash    TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL DEFAULT 0,
    indexed_at      TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (workspace_id, file_path)
);

CREATE INDEX IF NOT EXISTS idx_indexed_files_hash ON indexed_files(content_hash);
