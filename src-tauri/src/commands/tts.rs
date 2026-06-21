#[tauri::command]
pub async fn speak_word(_word_id: String) -> Result<(), String> {
    Err("TTS not yet implemented — Sprint 2".into())
}
