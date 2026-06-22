use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Deserialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, DbError>;

const SEED_EN: &str = include_str!("../../seed/vocabulary_en.json");
const SEED_JA: &str = include_str!("../../seed/vocabulary_ja.json");

#[derive(Debug, Deserialize)]
struct SeedEntry {
    word: String,
    reading: Option<String>,
    translation: String,
    language: String,
    difficulty: i64,
    pitch_pattern: Option<String>,
}

pub fn resolve_db_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("kotoba").join("kotoba.db")
}

pub async fn init_pool(db_path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let url = format!("sqlite:{}?mode=rwc", db_path.display());
    info!(path = %db_path.display(), "opening database");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("migrations applied");

    seed_vocabulary_if_empty(&pool).await?;

    Ok(pool)
}

async fn seed_vocabulary_if_empty(pool: &SqlitePool) -> Result<()> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vocabulary")
        .fetch_one(pool)
        .await?;

    if count.0 > 0 {
        info!(count = count.0, "vocabulary already seeded");
        return Ok(());
    }

    info!("seeding vocabulary from embedded JSON");
    seed_language(pool, SEED_EN, "en").await?;
    seed_language(pool, SEED_JA, "ja").await?;
    Ok(())
}

async fn seed_language(pool: &SqlitePool, json: &str, label: &str) -> Result<()> {
    let entries: Vec<SeedEntry> = serde_json::from_str(json)?;
    let count = entries.len();
    let now = Utc::now().to_rfc3339();

    for entry in entries {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO vocabulary
                (id, word, reading, translation, language, difficulty, pitch_pattern, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&entry.word)
        .bind(&entry.reading)
        .bind(&entry.translation)
        .bind(&entry.language)
        .bind(entry.difficulty)
        .bind(&entry.pitch_pattern)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    }

    info!(language = label, count, "seeded vocabulary entries");
    Ok(())
}

pub async fn is_onboarding_required(pool: &SqlitePool) -> Result<bool> {
    for name in crate::models::required_onboarding_models() {
        if !has_model(pool, name).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn count_model_manifest(pool: &SqlitePool) -> Result<i64> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM model_manifest")
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct VocabularyRow {
    pub id: String,
    pub word: String,
    pub reading: Option<String>,
    pub translation: String,
    pub language: String,
    pub difficulty: i64,
    pub pitch_pattern: Option<String>,
}

pub async fn get_random_word(pool: &SqlitePool, language: &str) -> Result<Option<VocabularyRow>> {
    let row = sqlx::query_as::<_, VocabularyRow>(
        r#"
        SELECT id, word, reading, translation, language, difficulty, pitch_pattern
        FROM vocabulary
        WHERE language = ?
        ORDER BY RANDOM()
        LIMIT 1
        "#,
    )
    .bind(language)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_vocabulary_by_id(pool: &SqlitePool, id: &str) -> Result<Option<VocabularyRow>> {
    let row = sqlx::query_as::<_, VocabularyRow>(
        r#"
        SELECT id, word, reading, translation, language, difficulty, pitch_pattern
        FROM vocabulary
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn insert_attempt(
    pool: &SqlitePool,
    vocabulary_id: &str,
    spoken_transcript: &str,
    score: f64,
    score_breakdown: &str,
    scoring_version: &str,
    audio_persisted: bool,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        r#"
        INSERT INTO attempt
            (id, vocabulary_id, spoken_transcript, score, score_breakdown, scoring_version,
             audio_persisted, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(vocabulary_id)
    .bind(spoken_transcript)
    .bind(score)
    .bind(score_breakdown)
    .bind(scoring_version)
    .bind(audio_persisted)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(id)
}

pub async fn list_attempts(pool: &SqlitePool, limit: i64) -> Result<Vec<AttemptRow>> {
    let rows = sqlx::query_as::<_, AttemptRow>(
        r#"
        SELECT id, vocabulary_id, spoken_transcript, score, score_breakdown, scoring_version,
               audio_persisted, created_at
        FROM attempt
        ORDER BY created_at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptRow {
    pub id: String,
    pub vocabulary_id: String,
    pub spoken_transcript: String,
    pub score: f64,
    pub score_breakdown: String,
    pub scoring_version: String,
    pub audio_persisted: bool,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ModelManifestRow {
    pub name: String,
    pub version: String,
    pub path: String,
    pub checksum_sha256: String,
    pub downloaded_at: String,
    pub latest_known_version: Option<String>,
    pub last_update_check_at: Option<String>,
}

pub async fn upsert_model_manifest(
    pool: &SqlitePool,
    name: &str,
    version: &str,
    path: &str,
    checksum_sha256: &str,
    downloaded_at: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO model_manifest
            (name, version, path, checksum_sha256, downloaded_at, last_update_check_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(name) DO UPDATE SET
            version = excluded.version,
            path = excluded.path,
            checksum_sha256 = excluded.checksum_sha256,
            downloaded_at = excluded.downloaded_at,
            last_update_check_at = excluded.last_update_check_at
        "#,
    )
    .bind(name)
    .bind(version)
    .bind(path)
    .bind(checksum_sha256)
    .bind(downloaded_at)
    .bind(downloaded_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn has_model(pool: &SqlitePool, name: &str) -> Result<bool> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT name FROM model_manifest WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await?;
    Ok(row.is_some())
}

pub async fn list_model_manifest(pool: &SqlitePool) -> Result<Vec<ModelManifestRow>> {
    let rows = sqlx::query_as::<_, ModelManifestRow>(
        r#"
        SELECT name, version, path, checksum_sha256, downloaded_at,
               latest_known_version, last_update_check_at
        FROM model_manifest
        ORDER BY name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn save_consent(pool: &SqlitePool, audio_persisted: bool) -> Result<()> {
    let value = audio_persisted.to_string();
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('audio_persisted', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
    )
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_audio_persisted(pool: &SqlitePool) -> Result<bool> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'audio_persisted'"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(val,)| val == "true").unwrap_or(false))
}

pub async fn get_practice_language(pool: &SqlitePool) -> Result<String> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'practice_language'"
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(val,)| val).unwrap_or_else(|| "ja".to_string()))
}

pub async fn save_practice_language(pool: &SqlitePool, language: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES ('practice_language', ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
    )
    .bind(language)
    .execute(pool)
    .await?;
    Ok(())
}
