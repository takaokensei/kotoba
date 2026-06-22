use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Vocabulary {
    pub id: String,
    pub word: String,
    pub reading: Option<String>,
    pub translation: String,
    pub language: String,
    pub difficulty: i64,
    pub pitch_pattern: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Attempt {
    pub id: String,
    pub vocabulary_id: String,
    pub spoken_transcript: String,
    pub score: f64,
    pub score_breakdown: String,
    pub scoring_version: String,
    pub audio_persisted: bool,
    pub tutor_feedback: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub duration_seconds: i64,
    pub words_practiced: i64,
    pub average_score: f64,
    pub started_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Telemetry {
    pub id: i64,
    pub attempt_id: Option<String>,
    pub stt_latency_ms: Option<i64>,
    pub scoring_latency_ms: Option<i64>,
    pub llm_latency_ms: Option<i64>,
    pub tts_latency_ms: Option<i64>,
    pub created_at: String,
}
