use serde::Serialize;
use sqlx::SqlitePool;
use tauri::{AppHandle, State};

use crate::db;
use crate::llm::daemon_waker;
use crate::models::{self, catalog};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrerequisiteStatus {
    pub ollama_available: bool,
    pub ollama_models: Vec<String>,
    pub ollama_wake_attempted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogEntry {
    pub name: String,
    pub version: String,
    pub size_mb_estimate: u32,
    pub required: bool,
    pub installed: bool,
}

#[tauri::command]
pub async fn check_prerequisites() -> Result<PrerequisiteStatus, String> {
    let (available, models) = daemon_waker::ensure_ollama_awake_with_models().await;

    Ok(PrerequisiteStatus {
        ollama_available: available,
        ollama_models: models,
        ollama_wake_attempted: true,
    })
}

#[tauri::command]
pub async fn is_onboarding_required(pool: State<'_, SqlitePool>) -> Result<bool, String> {
    db::is_onboarding_required(&pool)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_available_models(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<ModelCatalogEntry>, String> {
    let mut entries = Vec::new();
    let required: std::collections::HashSet<&str> =
        models::required_onboarding_models().iter().copied().collect();

    for name in ["whisper-tiny", "piper-en", "piper-ja"] {
        let def = catalog::get_model(name).ok_or_else(|| format!("unknown model {name}"))?;
        let installed = db::has_model(&pool, name).await.map_err(|e| e.to_string())?;
        entries.push(ModelCatalogEntry {
            name: def.name.to_string(),
            version: def.version.to_string(),
            size_mb_estimate: def.size_mb_estimate,
            required: required.contains(name),
            installed,
        });
    }

    Ok(entries)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    model_name: String,
) -> Result<ModelInfo, String> {
    let definition = catalog::get_model(&model_name)
        .ok_or_else(|| format!("modelo desconhecido: {model_name}"))?;

    let row = models::download_model(&app, &pool, definition)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ModelInfo {
        name: row.name,
        version: row.version,
        path: row.path,
        downloaded_at: row.downloaded_at,
        size_mb_estimate: Some(definition.size_mb_estimate),
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub name: String,
    pub version: String,
    pub path: String,
    pub downloaded_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_mb_estimate: Option<u32>,
}

#[tauri::command]
pub async fn get_model_manifest(pool: State<'_, SqlitePool>) -> Result<Vec<ModelInfo>, String> {
    let rows = db::list_model_manifest(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let size_mb = catalog::get_model(&r.name).map(|d| d.size_mb_estimate);
            ModelInfo {
                name: r.name,
                version: r.version,
                path: r.path,
                downloaded_at: r.downloaded_at,
                size_mb_estimate: size_mb,
            }
        })
        .collect())
}

#[tauri::command]
pub async fn check_for_updates(pool: State<'_, SqlitePool>) -> Result<Vec<ModelInfo>, String> {
    get_model_manifest(pool).await
}

#[tauri::command]
pub async fn save_consent(
    pool: State<'_, SqlitePool>,
    audio_persisted: bool,
) -> Result<(), String> {
    for name in models::required_onboarding_models() {
        if !db::has_model(&pool, name).await.map_err(|e| e.to_string())? {
            return Err(format!(
                "modelo obrigatório '{name}' não instalado — conclua o download antes de continuar"
            ));
        }
    }

    db::save_consent(&pool, audio_persisted)
        .await
        .map_err(|e| e.to_string())
}
