-- NexusIDE SQLite Schema v1
-- PRAGMAs applied at connection: WAL, foreign_keys=ON

CREATE TABLE IF NOT EXISTS workspaces (
    id              TEXT PRIMARY KEY,
    root_path       TEXT NOT NULL UNIQUE,
    name            TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS settings (
    key             TEXT PRIMARY KEY,
    value_json      TEXT NOT NULL,
    scope           TEXT NOT NULL DEFAULT 'global',
    workspace_id    TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_settings_scope ON settings(scope, workspace_id);

CREATE TABLE IF NOT EXISTS sessions (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
    title           TEXT,
    status          TEXT NOT NULL DEFAULT 'active',
    revision        INTEGER NOT NULL DEFAULT 0,
    model_id        TEXT,
    agent_profile   TEXT NOT NULL DEFAULT 'default',
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sessions_workspace ON sessions(workspace_id);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

CREATE TABLE IF NOT EXISTS messages (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    role            TEXT NOT NULL,
    content         TEXT NOT NULL,
    metadata_json   TEXT,
    parent_id       TEXT REFERENCES messages(id),
    sequence        INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_messages_session_seq
    ON messages(session_id, sequence);

CREATE TABLE IF NOT EXISTS checkpoints (
    id              TEXT PRIMARY KEY,
    session_id      TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    thread_id       TEXT NOT NULL,
    checkpoint_ns   TEXT NOT NULL DEFAULT '',
    checkpoint_id   TEXT NOT NULL,
    parent_id       TEXT,
    state_blob      BLOB NOT NULL,
    metadata_json   TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_session ON checkpoints(session_id, created_at DESC);

CREATE TABLE IF NOT EXISTS tool_definitions (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL UNIQUE,
    description     TEXT NOT NULL,
    schema_json     TEXT NOT NULL,
    source          TEXT NOT NULL DEFAULT 'builtin',
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS permission_policies (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    tool_name       TEXT,
    resource_pattern TEXT,
    action          TEXT NOT NULL,
    priority        INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS audit_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT REFERENCES sessions(id) ON DELETE SET NULL,
    tool_name       TEXT NOT NULL,
    arguments_json  TEXT,
    result_status   TEXT NOT NULL,
    approved_by     TEXT,
    duration_ms     INTEGER,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_session ON audit_log(session_id, created_at DESC);

CREATE TABLE IF NOT EXISTS code_chunks (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    file_path       TEXT NOT NULL,
    language        TEXT,
    symbol_name     TEXT,
    symbol_kind     TEXT,
    start_line      INTEGER NOT NULL,
    end_line        INTEGER NOT NULL,
    content_hash    TEXT NOT NULL,
    chroma_id       TEXT,
    indexed_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_chunks_workspace_file ON code_chunks(workspace_id, file_path);
CREATE INDEX IF NOT EXISTS idx_chunks_hash ON code_chunks(content_hash);

CREATE TABLE IF NOT EXISTS index_jobs (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    status          TEXT NOT NULL DEFAULT 'pending',
    files_total     INTEGER DEFAULT 0,
    files_done      INTEGER DEFAULT 0,
    error_message   TEXT,
    started_at      TEXT,
    finished_at     TEXT
);

CREATE TABLE IF NOT EXISTS sync_cursors (
    client_id       TEXT PRIMARY KEY,
    last_event_id   INTEGER NOT NULL DEFAULT 0,
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sync_events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id      TEXT REFERENCES sessions(id) ON DELETE CASCADE,
    event_type      TEXT NOT NULL,
    payload_json    TEXT NOT NULL,
    revision        INTEGER NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_sync_events_session ON sync_events(session_id, id);
