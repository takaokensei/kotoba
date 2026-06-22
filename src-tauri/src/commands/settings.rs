use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

use crate::db;

#[tauri::command]
pub async fn delete_model(_model_name: String) -> Result<(), String> {
    Err("Model deletion not yet implemented — Sprint 1 Task 1.2".into())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub audio_persisted: bool,
    pub practice_language: String,
}

#[tauri::command]
pub async fn get_settings(pool: State<'_, SqlitePool>) -> Result<AppSettings, String> {
    let audio_persisted = db::get_audio_persisted(&pool)
        .await
        .map_err(|e| e.to_string())?;
    let practice_language = db::get_practice_language(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(AppSettings {
        audio_persisted,
        practice_language,
    })
}

#[tauri::command]
pub async fn update_settings(
    pool: State<'_, SqlitePool>,
    settings: AppSettings,
) -> Result<(), String> {
    db::save_consent(&pool, settings.audio_persisted)
        .await
        .map_err(|e| e.to_string())?;
    db::save_practice_language(&pool, &settings.practice_language)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
