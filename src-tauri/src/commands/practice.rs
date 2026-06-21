use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

use crate::db;
use crate::scoring::composition::{self, ScoreBreakdown};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PracticeWord {
    pub id: String,
    pub word: String,
    pub reading: Option<String>,
    pub translation: String,
    pub language: String,
    pub difficulty: i64,
    pub pitch_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttemptResult {
    pub id: String,
    pub transcription: String,
    pub score: f64,
    pub score_breakdown: ScoreBreakdown,
    pub scoring_version: String,
}

#[tauri::command]
pub async fn get_next_word(
    pool: State<'_, SqlitePool>,
    language: String,
) -> Result<PracticeWord, String> {
    let row = db::get_random_word(&pool, &language)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no vocabulary found for language '{language}'"))?;

    Ok(PracticeWord {
        id: row.id,
        word: row.word,
        reading: row.reading,
        translation: row.translation,
        language: row.language,
        difficulty: row.difficulty,
        pitch_pattern: row.pitch_pattern,
    })
}

#[tauri::command]
pub async fn score_attempt(
    pool: State<'_, SqlitePool>,
    vocabulary_id: String,
    transcript: String,
) -> Result<AttemptResult, String> {
    let vocab = db::get_vocabulary_by_id(&pool, &vocabulary_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "vocabulary not found".to_string())?;

    let target = vocab.reading.as_deref().unwrap_or(&vocab.word);
    let (score, breakdown, version) = composition::compose_v1(target, &transcript);
    let breakdown_json =
        serde_json::to_string(&breakdown).map_err(|e| e.to_string())?;

    let attempt_id = db::insert_attempt(
        &pool,
        &vocabulary_id,
        &transcript,
        score,
        &breakdown_json,
        version,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(AttemptResult {
        id: attempt_id,
        transcription: transcript,
        score,
        score_breakdown: breakdown,
        scoring_version: version.to_string(),
    })
}

#[tauri::command]
pub async fn record_and_transcribe(_max_duration_ms: u64) -> Result<String, String> {
    Err("STT not yet implemented — Sprint 1 Task 1.3".into())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TutorFeedback {
    pub text: String,
    pub corrections: Vec<Correction>,
    pub llm_unavailable: bool,
}

#[derive(Debug, Serialize)]
pub struct Correction {
    pub r#type: String,
    pub r#where: String,
    pub what: String,
    pub how: String,
}

#[tauri::command]
pub async fn get_tutor_feedback(
    _vocabulary_id: String,
    attempt_result: AttemptResult,
) -> Result<TutorFeedback, String> {
    Ok(TutorFeedback {
        text: format!(
            "Score determinístico: {:.0}/100. Feedback do tutor disponível após integração com Ollama.",
            attempt_result.score
        ),
        corrections: vec![],
        llm_unavailable: true,
    })
}

#[tauri::command]
pub async fn list_recent_attempts(
    pool: State<'_, SqlitePool>,
    limit: Option<i64>,
) -> Result<Vec<db::AttemptRow>, String> {
    db::list_attempts(&pool, limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}
