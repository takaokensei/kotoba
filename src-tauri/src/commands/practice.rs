use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

use crate::db;
use crate::scoring::composition::{self, ScoreBreakdown};
use crate::audio::{capture, sidecar_lifecycle};

static CURRENT_TARGET_WORD: Mutex<Option<String>> = Mutex::new(None);

fn resolve_recordings_dir() -> std::path::PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    base.join("kotoba").join("recordings")
}

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

    // Store target word for the mock sidecar CLI
    let word_text = row.reading.clone().unwrap_or_else(|| row.word.clone());
    if let Ok(mut current) = CURRENT_TARGET_WORD.lock() {
        *current = Some(word_text);
    }

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

    // Check if user consented to persist audio
    let audio_persisted = db::get_audio_persisted(&pool).await.unwrap_or(false);

    let attempt_id = db::insert_attempt(
        &pool,
        &vocabulary_id,
        &transcript,
        score,
        &breakdown_json,
        version,
        audio_persisted,
    )
    .await
    .map_err(|e| e.to_string())?;

    if audio_persisted {
        if let Some(wav_bytes) = capture::take_last_recorded_wav() {
            let recordings_dir = resolve_recordings_dir();
            if let Err(e) = std::fs::create_dir_all(&recordings_dir) {
                tracing::error!("Falha ao criar diretório de gravações: {e}");
            } else {
                let file_path = recordings_dir.join(format!("{attempt_id}.wav"));
                if let Err(e) = std::fs::write(&file_path, wav_bytes) {
                    tracing::error!("Falha ao gravar arquivo de áudio: {e}");
                } else {
                    tracing::info!(path = %file_path.display(), "Áudio gravado com sucesso");
                }
            }
        }
    } else {
        capture::clear_last_recorded_wav();
    }

    Ok(AttemptResult {
        id: attempt_id,
        transcription: transcript,
        score,
        score_breakdown: breakdown,
        scoring_version: version.to_string(),
    })
}

#[tauri::command]
pub async fn record_and_transcribe(
    pool: State<'_, SqlitePool>,
    app: tauri::AppHandle,
    max_duration_ms: u64,
) -> Result<String, String> {
    // 1. Capture microphone audio
    let wav_bytes_opt = capture::capture_mic_audio(&app, max_duration_ms).await?;
    
    let wav_bytes = match wav_bytes_opt {
        Some(bytes) => bytes,
        None => return Ok("".to_string()), // Cancelled or empty
    };
    
    // Save to global state so score_attempt can write it if consent is true
    capture::set_last_recorded_wav(wav_bytes.clone());
    
    // 2. Get whisper model path from model manifest
    let manifest = db::list_model_manifest(&pool)
        .await
        .map_err(|e| e.to_string())?;
        
    let whisper_model = manifest
        .iter()
        .find(|m| m.name == "whisper-tiny")
        .ok_or_else(|| "Modelo 'whisper-tiny' não está instalado. Por favor, conclua o onboarding.".to_string())?;
        
    // 3. Set environment variable for mock sidecar if target word exists
    let target_word = CURRENT_TARGET_WORD.lock().unwrap().clone().unwrap_or_default();
    std::env::set_var("KOTOBA_MOCK_TRANSCRIPTION", &target_word);
    
    let transcript = sidecar_lifecycle::run_whisper_transcription(&app, &whisper_model.path, &wav_bytes).await?;
    
    Ok(transcript)
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
