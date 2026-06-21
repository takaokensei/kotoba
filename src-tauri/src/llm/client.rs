//! HTTP client for Ollama at localhost:11434.

use std::time::Duration;

const OLLAMA_BASE: &str = "http://localhost:11434";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

pub async fn is_available() -> bool {
    match fetch_tags().await {
        Ok(_) => true,
        Err(e) => {
            tracing::debug!(error = %e, "Ollama unavailable");
            false
        }
    }
}

pub async fn list_models() -> Result<Vec<String>, String> {
    let body = fetch_tags().await?;
    let names = body
        .models
        .into_iter()
        .map(|m| m.name)
        .collect();
    Ok(names)
}

async fn fetch_tags() -> Result<OllamaTagsResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(format!("{OLLAMA_BASE}/api/tags"))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("Ollama returned HTTP {}", response.status()));
    }

    response
        .json::<OllamaTagsResponse>()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct OllamaModelEntry {
    name: String,
}
