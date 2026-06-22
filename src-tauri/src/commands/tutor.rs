//! Tutor feedback command using local Ollama LLM.

use sqlx::SqlitePool;
use tauri::State;
use serde::{Serialize, Deserialize};
use crate::db;

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
    // Attempt to query Ollama tags to find a suitable local model
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
    // Fallback model if Ollama is not responding or no tags are found
    "llama3:latest".to_string()
}

/// Generates personalized pedagogical feedback using a local Ollama LLM.
#[tauri::command]
pub async fn generate_tutor_feedback(
    pool: State<'_, SqlitePool>,
    word_id: String,
    user_transcription: String,
    score: i32,
) -> Result<String, String> {
    // 1. Fetch vocabulary row
    let row = db::get_vocabulary_by_id(&pool, &word_id)
        .await
        .map_err(|e| format!("Erro no banco de dados: {e}"))?
        .ok_or_else(|| format!("Palavra não encontrada: {word_id}"))?;

    // 2. Select appropriate system prompt based on language
    let system_prompt = if row.language == "ja" {
        "Você é um tutor de japonês atencioso, empático e detalhista. Seu objetivo é ajudar um estudante brasileiro a melhorar a pronúncia das palavras.\n\
         Compare a palavra original e sua leitura (geralmente em hiragana/romaji) com a transcrição que o aluno pronunciou.\n\
         Dê dicas práticas de pronúncia em português brasileiro, focando nas diferenças entre o som produzido pelo aluno e a pronúncia correta (como alongamento de vogais, pronúncia de consoantes duplas/pequeno tsu, ou padrão de entonação/pitch-accent se aplicável).\n\
         Seja breve e construtivo. Ignore pontuações como pontos finais, exclamação ou interrogação no texto transcrito. Se o score for baixo, aponte diretamente onde o aluno errou e como praticar."
    } else {
        "Você é um tutor de inglês atencioso, empático e detalhista. Seu objetivo é ajudar um estudante brasileiro a melhorar a pronúncia das palavras.\n\
         Compare a palavra original com a transcrição que o aluno pronunciou.\n\
         Dê dicas práticas de pronúncia em português brasileiro, focando em erros comuns de brasileiros (como o som do 'r' ou 'l' final, vogais curtas/longas, ou o acréscimo de vogais no final de palavras terminadas em consoantes).\n\
         Seja breve e construtivo. Ignore pontuações no texto transcrito. Se o score for baixo, aponte diretamente onde o aluno errou e como praticar."
    };

    // 3. Construct the prompt context
    let prompt = format!(
        "Idioma: {}\n\
         Palavra-alvo: {}\n\
         Leitura/Pronúncia de referência: {}\n\
         Tradução: {}\n\
         Padrão de Pitch (Entonação): {}\n\
         Transcrição obtida do áudio do aluno: {}\n\
         Score de similaridade determinístico (0-100): {}\n\n\
         Com base nos dados acima, forneça um feedback construtivo e dicas de pronúncia em português brasileiro.",
        if row.language == "ja" { "Japonês" } else { "Inglês" },
        row.word,
        row.reading.as_deref().unwrap_or(&row.word),
        row.translation,
        row.pitch_pattern.as_deref().unwrap_or("não especificado"),
        user_transcription,
        score
    );

    // 4. Determine which model to use
    let model = get_tutor_model().await;

    // 5. Send HTTP request to local Ollama instance
    let client = reqwest::Client::new();
    let body = OllamaRequest {
        model,
        prompt,
        system: system_prompt.to_string(),
        stream: false,
    };

    let response = client.post("http://localhost:11434/api/generate")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Não foi possível conectar ao Ollama: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Erro retornado pelo Ollama: {}", response.status()));
    }

    let res_body: OllamaResponse = response.json()
        .await
        .map_err(|e| format!("Falha ao decodificar resposta do Ollama: {e}"))?;

    Ok(res_body.response)
}
