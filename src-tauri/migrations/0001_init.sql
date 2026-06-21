CREATE TABLE vocabulary (
    id TEXT PRIMARY KEY,
    word TEXT NOT NULL,
    reading TEXT,
    translation TEXT NOT NULL,
    language TEXT NOT NULL,
    difficulty INTEGER NOT NULL DEFAULT 1,
    pitch_pattern TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE attempt (
    id TEXT PRIMARY KEY,
    vocabulary_id TEXT NOT NULL REFERENCES vocabulary(id),
    spoken_transcript TEXT NOT NULL,
    score REAL NOT NULL,
    score_breakdown TEXT NOT NULL,
    scoring_version TEXT NOT NULL,
    audio_persisted BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE session (
    id TEXT PRIMARY KEY,
    duration_seconds INTEGER NOT NULL,
    words_practiced INTEGER NOT NULL,
    average_score REAL NOT NULL,
    started_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_attempt_vocabulary_id ON attempt(vocabulary_id);
CREATE INDEX idx_attempt_created_at ON attempt(created_at);

CREATE TABLE model_manifest (
    name TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    path TEXT NOT NULL,
    checksum_sha256 TEXT NOT NULL,
    downloaded_at TEXT NOT NULL,
    latest_known_version TEXT,
    last_update_check_at TEXT
);

CREATE TABLE telemetry (
    id INTEGER PRIMARY KEY,
    attempt_id TEXT REFERENCES attempt(id),
    stt_latency_ms INTEGER,
    scoring_latency_ms INTEGER,
    llm_latency_ms INTEGER,
    tts_latency_ms INTEGER,
    created_at TEXT NOT NULL
);
