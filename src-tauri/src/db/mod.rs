use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Deserialize;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;
use uuid::Uuid;

pub mod models;

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
    let count_en = seed_language(pool, SEED_EN, "en").await?;
    let count_ja = seed_language(pool, SEED_JA, "ja").await?;
    info!(total_seeded = count_en + count_ja, "Vocabulary seeding completed successfully on first boot");
    Ok(())
}

async fn seed_language(pool: &SqlitePool, json: &str, label: &str) -> Result<usize> {
    let entries: Vec<SeedEntry> = serde_json::from_str(json)?;
    let count = entries.len();
    let now = Utc::now().to_rfc3339();

    for entry in &entries {
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
    Ok(count)
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
    let id = Uuid::now_v7().to_string();
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
        SELECT 
            a.id, 
            a.vocabulary_id, 
            a.spoken_transcript, 
            a.score, 
            a.score_breakdown, 
            a.scoring_version,
            a.audio_persisted, 
            a.tutor_feedback, 
            a.created_at,
            v.word,
            v.reading,
            v.translation
        FROM attempt a
        JOIN vocabulary v ON a.vocabulary_id = v.id
        ORDER BY a.created_at DESC
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
    pub tutor_feedback: Option<String>,
    pub created_at: String,
    pub word: String,
    pub reading: Option<String>,
    pub translation: String,
}

/// Persists the validated tutor feedback text for an existing attempt row.
///
/// Called after the Honesty Gate has inspected the LLM response so we only
/// ever store the final, gate-approved (or fallback) text.
pub async fn update_attempt_feedback(
    pool: &SqlitePool,
    attempt_id: &str,
    feedback: &str,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE attempt
        SET tutor_feedback = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(feedback)
    .bind(&now)
    .bind(attempt_id)
    .execute(pool)
    .await?;
    Ok(())
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

// ─── Repository Functions for Session, Telemetry, and Attempt ───

pub async fn get_attempt_by_id(pool: &SqlitePool, id: &str) -> Result<Option<models::Attempt>> {
    let row = sqlx::query_as::<_, models::Attempt>(
        r#"
        SELECT id, vocabulary_id, spoken_transcript, score, score_breakdown, scoring_version,
               audio_persisted, tutor_feedback, created_at, updated_at
        FROM attempt
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn insert_vocabulary(
    pool: &SqlitePool,
    word: &str,
    reading: Option<&str>,
    translation: &str,
    language: &str,
    difficulty: i64,
    pitch_pattern: Option<&str>,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO vocabulary (id, word, reading, translation, language, difficulty, pitch_pattern, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(word)
    .bind(reading)
    .bind(translation)
    .bind(language)
    .bind(difficulty)
    .bind(pitch_pattern)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update_vocabulary(
    pool: &SqlitePool,
    id: &str,
    word: &str,
    reading: Option<&str>,
    translation: &str,
    difficulty: i64,
    pitch_pattern: Option<&str>,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE vocabulary
        SET word = ?, reading = ?, translation = ?, difficulty = ?, pitch_pattern = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(word)
    .bind(reading)
    .bind(translation)
    .bind(difficulty)
    .bind(pitch_pattern)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_vocabulary(pool: &SqlitePool, id: &str) -> Result<Option<models::Vocabulary>> {
    let row = sqlx::query_as::<_, models::Vocabulary>(
        r#"
        SELECT id, word, reading, translation, language, difficulty, pitch_pattern, created_at, updated_at
        FROM vocabulary
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn insert_session(
    pool: &SqlitePool,
    duration_seconds: i64,
    words_practiced: i64,
    average_score: f64,
) -> Result<String> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        INSERT INTO session (id, duration_seconds, words_practiced, average_score, started_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(duration_seconds)
    .bind(words_practiced)
    .bind(average_score)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn update_session(
    pool: &SqlitePool,
    id: &str,
    duration_seconds: i64,
    words_practiced: i64,
    average_score: f64,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        r#"
        UPDATE session
        SET duration_seconds = ?, words_practiced = ?, average_score = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(duration_seconds)
    .bind(words_practiced)
    .bind(average_score)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_session_by_id(pool: &SqlitePool, id: &str) -> Result<Option<models::Session>> {
    let row = sqlx::query_as::<_, models::Session>(
        r#"
        SELECT id, duration_seconds, words_practiced, average_score, started_at, updated_at
        FROM session
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Increments `words_practiced`, recalculates `average_score` as a rolling
/// mean, recomputes `duration_seconds` from `started_at`, and refreshes
/// `updated_at`.  This is the single write path for in-flight session updates.
pub async fn record_session_activity(
    pool: &SqlitePool,
    session_id: &str,
    score: f64,
) -> Result<()> {
    use tracing::warn;

    let session = match get_session_by_id(pool, session_id).await? {
        Some(s) => s,
        None => {
            warn!(session_id, "record_session_activity: session not found — skipping update");
            return Ok(());
        }
    };

    let now = Utc::now();
    let now_str = now.to_rfc3339();

    // Recompute duration from started_at (best-effort; fallback to existing value)
    let elapsed_secs = chrono::DateTime::parse_from_rfc3339(&session.started_at)
        .map(|started| (now - started.with_timezone(&Utc)).num_seconds().max(0))
        .unwrap_or(session.duration_seconds);

    // Rolling average: (prev_avg * prev_count + new_score) / new_count
    let new_count = session.words_practiced + 1;
    let new_avg = (session.average_score * session.words_practiced as f64 + score)
        / new_count as f64;

    sqlx::query(
        r#"
        UPDATE session
        SET duration_seconds = ?, words_practiced = ?, average_score = ?, updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(elapsed_secs)
    .bind(new_count)
    .bind(new_avg)
    .bind(&now_str)
    .bind(session_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_telemetry(
    pool: &SqlitePool,
    attempt_id: Option<&str>,
    stt_latency_ms: Option<i64>,
    scoring_latency_ms: Option<i64>,
    llm_latency_ms: Option<i64>,
    tts_latency_ms: Option<i64>,
) -> Result<i64> {
    let now = Utc::now().to_rfc3339();
    let res = sqlx::query(
        r#"
        INSERT INTO telemetry (attempt_id, stt_latency_ms, scoring_latency_ms, llm_latency_ms, tts_latency_ms, created_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(attempt_id)
    .bind(stt_latency_ms)
    .bind(scoring_latency_ms)
    .bind(llm_latency_ms)
    .bind(tts_latency_ms)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

pub async fn get_telemetry_by_id(pool: &SqlitePool, id: i64) -> Result<Option<models::Telemetry>> {
    let row = sqlx::query_as::<_, models::Telemetry>(
        r#"
        SELECT id, attempt_id, stt_latency_ms, scoring_latency_ms, llm_latency_ms, tts_latency_ms, created_at
        FROM telemetry
        WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[tokio::test]
    async fn test_database_access_layer_and_updates() {
        // Create an in-memory SQLite database pool for testing
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Run migrations on the in-memory database
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // 1. Test Seeding (in-memory will trigger seeding since count is 0)
        seed_vocabulary_if_empty(&pool).await.unwrap();
        let seed_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM vocabulary")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(seed_count.0 > 0, "Seeding count should be greater than zero");

        // 2. Test Vocabulary Insert and Retrieval
        let vocab_id = insert_vocabulary(
            &pool,
            "テスト",
            Some("てすと"),
            "test",
            "ja",
            1,
            Some("heiban"),
        )
        .await
        .unwrap();

        let vocab = get_vocabulary(&pool, &vocab_id).await.unwrap().unwrap();
        assert_eq!(vocab.word, "テスト");
        assert_eq!(vocab.translation, "test");
        assert_eq!(vocab.created_at, vocab.updated_at);

        // Wait a brief moment to ensure time difference if any
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 3. Test Vocabulary Update (Enforce updated_at programmatic recalculation)
        update_vocabulary(
            &pool,
            &vocab_id,
            "テスト改",
            Some("てすとかい"),
            "test modified",
            2,
            Some("atamadaka"),
        )
        .await
        .unwrap();

        let updated_vocab = get_vocabulary(&pool, &vocab_id).await.unwrap().unwrap();
        assert_eq!(updated_vocab.word, "テスト改");
        assert_eq!(updated_vocab.difficulty, 2);
        assert_ne!(updated_vocab.created_at, updated_vocab.updated_at, "updated_at should be updated");

        // 4. Test Session Insert, Update and Retrieval
        let session_id = insert_session(&pool, 300, 5, 85.5).await.unwrap();
        let session = get_session_by_id(&pool, &session_id).await.unwrap().unwrap();
        assert_eq!(session.duration_seconds, 300);
        assert_eq!(session.words_practiced, 5);
        assert_eq!(session.average_score, 85.5);
        assert_eq!(session.started_at, session.updated_at);

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        update_session(&pool, &session_id, 350, 6, 88.0).await.unwrap();
        let updated_session = get_session_by_id(&pool, &session_id).await.unwrap().unwrap();
        assert_eq!(updated_session.duration_seconds, 350);
        assert_eq!(updated_session.words_practiced, 6);
        assert_eq!(updated_session.average_score, 88.0);
        assert_ne!(updated_session.started_at, updated_session.updated_at, "Session updated_at should be updated");

        // 5. Test Telemetry Insert and Retrieval
        let telemetry_id = insert_telemetry(&pool, None, Some(100), Some(50), Some(200), Some(150)).await.unwrap();
        let telemetry = get_telemetry_by_id(&pool, telemetry_id).await.unwrap().unwrap();
        assert_eq!(telemetry.stt_latency_ms, Some(100));
        assert_eq!(telemetry.scoring_latency_ms, Some(50));
        assert_eq!(telemetry.llm_latency_ms, Some(200));
        assert_eq!(telemetry.tts_latency_ms, Some(150));
    }

    #[tokio::test]
    async fn test_record_session_activity_rolling_average() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Create a fresh empty session
        let session_id = insert_session(&pool, 0, 0, 0.0).await.unwrap();

        // First attempt: score 80.0  →  avg = 80.0, count = 1
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        record_session_activity(&pool, &session_id, 80.0).await.unwrap();
        let s1 = get_session_by_id(&pool, &session_id).await.unwrap().unwrap();
        assert_eq!(s1.words_practiced, 1);
        assert!((s1.average_score - 80.0).abs() < 0.001, "avg after 1st attempt should be 80");
        assert!(s1.duration_seconds >= 0);

        // Second attempt: score 60.0  →  avg = (80+60)/2 = 70.0, count = 2
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        record_session_activity(&pool, &session_id, 60.0).await.unwrap();
        let s2 = get_session_by_id(&pool, &session_id).await.unwrap().unwrap();
        assert_eq!(s2.words_practiced, 2);
        assert!((s2.average_score - 70.0).abs() < 0.001, "avg after 2nd attempt should be 70");

        // Duration should be >= previous (monotonically non-decreasing)
        assert!(s2.duration_seconds >= s1.duration_seconds);
    }
}

