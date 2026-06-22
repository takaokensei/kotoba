//! Tutor feedback command using local Ollama LLM.
//!
//! ## Pipeline (Task 3.2 + 3.3)
//! 1. Fetch vocabulary row from SQLite.
//! 2. Build language-tailored system prompt + user prompt.
//! 3. Call local Ollama HTTP endpoint.
//! 4. **Honesty Gate** intercepts the response — replaces false praise with a
//!    deterministic fallback if the score does not support it.
//! 5. **Persist** the gate-approved feedback text back to the `attempt` row.

use sqlx::SqlitePool;
use tauri::State;
use serde::{Serialize, Deserialize};
use tracing::{info, warn};

use crate::db;
use crate::llm::honesty_gate::{self, GateVerdict};
use crate::llm::daemon_waker;
use crate::scoring::composition::ScoreBreakdown;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

async fn get_tutor_model() -> String {
    // Attempt to query Ollama tags to find a suitable local model.
    match reqwest::get("http://localhost:11434/api/tags").await {
        Ok(resp) => {
            #[derive(Deserialize)]
            struct TagModel { name: String }
            #[derive(Deserialize)]
            struct TagsResponse { models: Vec<TagModel> }

            if let Ok(tags) = resp.json::<TagsResponse>().await {
                for m in tags.models {
                    if !m.name.contains("embed") {
                        return m.name;
                    }
                }
            }
        }
        Err(_) => {}
    }
    // Fallback model if Ollama is not responding or no tags are found.
    "llama3:latest".to_string()
}

/// Generates personalized pedagogical feedback using a local Ollama LLM.
///
/// The feedback is validated by the Honesty Gate before being returned to the
/// frontend and persisted into the `attempt` row identified by `attempt_id`.
///
/// # Parameters
/// - `word_id`            – UUID of the vocabulary word being practiced.
/// - `attempt_id`         – UUID of the attempt row created by `score_attempt`.
/// - `user_transcription` – Microphone transcript (already normalised).
/// - `score`              – Deterministic similarity score (0 – 100).
#[tauri::command]
pub async fn generate_tutor_feedback(
    pool: State<'_, SqlitePool>,
    word_id: String,
    attempt_id: String,
    user_transcription: String,
    score: i32,
) -> Result<String, String> {
    // 1. Fetch vocabulary row.
    let row = db::get_vocabulary_by_id(&pool, &word_id)
        .await
        .map_err(|e| format!("Erro no banco de dados: {e}"))?
        .ok_or_else(|| format!("Palavra não encontrada: {word_id}"))?;

    // ── Perfect Score Protocol (Task 4.4) ──
    // If the user achieved a flawless 100% score, immediately validate and congratulate
    // without calling the LLM to prevent any possible hallucinations.
    if score == 100 {
        let perfect_feedback = "Parabéns! Sua pronúncia foi perfeita e idêntica à de um falante nativo. Excelente trabalho!";
        if !attempt_id.is_empty() {
            let _ = db::update_attempt_feedback(&pool, &attempt_id, perfect_feedback).await;
        }
        return Ok(perfect_feedback.to_string());
    }

    // 2. Select appropriate system prompt based on language with strict guardrails (Task 4.4 / Task 5.1).
    let system_prompt = if row.language == "ja" {
        "Você é um tutor de japonês atencioso, empático e detalhista. Seu objetivo é ajudar um estudante brasileiro a melhorar a pronúncia das palavras.\n\
         [DIRETRIZES ABSOLUTAS E OBRIGATÓRIAS]\n\
         - ELIMINAÇÃO DE JARGÃO: É estritamente proibido usar jargões fonéticos complexos como \"alveolar\", \"bilabial\", \"velar\" ou \"fricativa\" (e suas variações). Explique como posicionar a língua e os lábios de forma simples e leiga (exemplo: coloque a ponta da língua atrás dos dentes de cima).\n\
         - FOCO PRÁTICO: Dê dicas totalmente focadas na mecânica real: postura da boca/lábios, pausas para respiração (especialmente antes de consoantes duplas/sokuon) e ritmo/tempo da fala comparando com a palavra original.\n\
         - DIRETRIZ CRÍTICA DE ALINHAMENTO: Você receberá um detalhamento de notas (score breakdown) contendo 'text_score' (similaridade ortográfica) e 'phonetic_score' (similaridade fonética/de pronúncia). Se 'phonetic_score' for 100% ou muito próximo disso (ex: >= 90%), a pronúncia do usuário foi matematicamente perfeita. Se o score total ou o 'text_score' for baixo, isso se deve estritamente a uma discrepância ortográfica (exemplo: Kanji vs. Hiragana). Nesse cenário, NÃO alucine erros anatômicos, de respiração ou de posicionamento da língua. Em vez disso, parabenize a pronúncia impecável e note gentilmente a variação de escrita.\n\
         Seja breve e construtivo. Ignore pontuações como pontos finais ou exclamações na transcrição obtida."
    } else {
        "Você é um tutor de inglês atencioso, empático e detalhista. Seu objetivo é ajudar um estudante brasileiro a melhorar a pronúncia das palavras.\n\
         [DIRETRIZES ABSOLUTAS E OBRIGATÓRIAS]\n\
         - ELIMINAÇÃO DE JARGÃO: É estritamente proibido usar jargões fonéticos complexos como \"alveolar\", \"bilabial\", \"velar\" ou \"fricativa\" (e suas variações). Explique como posicionar a língua e os lábios de forma simples e leiga.\n\
         - FOCO PRÁTICO: Dê dicas totalmente focadas na mecânica real: postura da boca/lábios, pausas para respiração e ritmo/tempo da fala comparando com a palavra original.\n\
         - DIRETRIZ CRÍTICA DE ALINHAMENTO: Você receberá um detalhamento de notas (score breakdown) contendo 'text_score' (similaridade ortográfica) e 'phonetic_score' (similaridade fonética/de pronúncia). Se 'phonetic_score' for 100% ou muito próximo disso (ex: >= 90%), a pronúncia do usuário foi matematicamente perfeita. Se o score total ou o 'text_score' for baixo, isso se deve estritamente a uma discrepância de digitação ou representação escrita. Nesse cenário, NÃO alucine erros anatômicos, de respiração ou de posicionamento da língua. Em vez disso, parabenize a pronúncia impecável e note gentilmente a variação de escrita.\n\
         Seja breve e construtivo. Ignore pontuações como pontos finais ou exclamações na transcrição obtida."
    };

    // Extract score breakdown components (Task 5.1)
    let mut text_score = score as f64 / 100.0;
    let mut phonetic_score = None;

    if !attempt_id.is_empty() {
        if let Ok(Some(breakdown_str)) = sqlx::query_scalar::<_, String>(
            "SELECT score_breakdown FROM attempt WHERE id = ?"
        )
        .bind(&attempt_id)
        .fetch_optional(pool.inner())
        .await
        {
            if let Ok(breakdown) = serde_json::from_str::<ScoreBreakdown>(&breakdown_str) {
                text_score = breakdown.text;
                phonetic_score = breakdown.phonetic;
            }
        }
    }

    let phonetic_score_str = match phonetic_score {
        Some(s) => format!("{:.0}%", s * 100.0),
        None => "não aplicável (V1)".to_string(),
    };

    // 3. Construct the prompt context.
    let prompt = format!(
        "Idioma: {}\n\
         Palavra-alvo: {}\n\
         Leitura/Pronúncia de referência: {}\n\
         Tradução: {}\n\
         Padrão de Pitch (Entonação): {}\n\
         Transcrição obtida do áudio do aluno: {}\n\
         Score Total (0-100): {}\n\
         Score Breakdown:\n\
         - text_score (similaridade de escrita): {:.0}%\n\
         - phonetic_score (similaridade de pronúncia): {}\n\n\
         Com base nos dados acima, forneça um feedback construtivo e dicas de pronúncia em português brasileiro.",
        if row.language == "ja" { "Japonês" } else { "Inglês" },
        row.word,
        row.reading.as_deref().unwrap_or(&row.word),
        row.translation,
        row.pitch_pattern.as_deref().unwrap_or("não especificado"),
        user_transcription,
        score,
        text_score * 100.0,
        phonetic_score_str
    );

    // 4. Determine which model to use.
    let _ = daemon_waker::ensure_ollama_awake().await;
    let model = get_tutor_model().await;
    info!(model = %model, "Requesting tutor feedback from Ollama");

    // 5. Send HTTP request to local Ollama instance.
    let client = reqwest::Client::new();
    let body = OllamaRequest {
        model,
        prompt,
        system: system_prompt.to_string(),
        stream: false,
    };

    let raw_text = match client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            match response.json::<OllamaResponse>().await {
                Ok(res_body) => res_body.response,
                Err(e) => {
                    return Err(format!("Falha ao decodificar resposta do Ollama: {e}"))
                }
            }
        }
        Ok(response) => {
            return Err(format!("Erro retornado pelo Ollama: {}", response.status()))
        }
        Err(e) => {
            return Err(format!("Não foi possível conectar ao Ollama: {e}"))
        }
    };

    // 6. ── HONESTY GATE ────────────────────────────────────────────────────────
    // The gate inspects the LLM output and, if false praise is detected, replaces
    // it with a calibrated deterministic fallback. This is a hard code-level
    // guardrail — it cannot be overridden by prompt injection.
    let reading = row.reading.as_deref().unwrap_or(&row.word);
    let (validated_text, verdict) =
        honesty_gate::validate_feedback(score as f64, &raw_text, &row.word, reading);

    if verdict == GateVerdict::FallbackApplied {
        warn!(
            score = score,
            word = %row.word,
            "Honesty Gate triggered: LLM praised low-scoring attempt — fallback applied"
        );
    } else {
        info!(score = score, word = %row.word, "Honesty Gate: feedback passed");
    }

    // 7. ── PERSIST (Task 3.2) ─────────────────────────────────────────────────
    // Update the existing attempt row with the gate-approved feedback text so
    // the history view can display it without calling the LLM again.
    if !attempt_id.is_empty() {
        if let Err(e) =
            db::update_attempt_feedback(&pool, &attempt_id, &validated_text).await
        {
            // Non-fatal: log and continue — the user still gets the feedback text.
            warn!(attempt_id = %attempt_id, error = %e, "Failed to persist tutor feedback");
        } else {
            info!(attempt_id = %attempt_id, "Tutor feedback persisted to attempt row");
        }
    }

    Ok(validated_text)
}
