use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tauri::State;

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
pub async fn get_settings(_pool: State<'_, SqlitePool>) -> Result<AppSettings, String> {
    Ok(AppSettings {
        audio_persisted: false,
        practice_language: "ja".into(),
    })
}

#[tauri::command]
pub async fn update_settings(_settings: AppSettings) -> Result<(), String> {
    Ok(())
}
