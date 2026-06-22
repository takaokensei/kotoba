use std::path::Path;

pub trait SttEngine: Send + Sync {
    async fn transcribe(&self, audio_path: &Path) -> Result<String, String>;
}

pub struct WhisperEngine {
    pub app: tauri::AppHandle,
    pub model_path: String,
    pub language: String,
}

impl SttEngine for WhisperEngine {
    async fn transcribe(&self, audio_path: &Path) -> Result<String, String> {
        let wav_bytes = std::fs::read(audio_path)
            .map_err(|e| format!("Falha ao ler arquivo de áudio para transcrição: {e}"))?;
        
        crate::audio::sidecar_lifecycle::run_whisper_transcription(
            &self.app,
            &self.model_path,
            &self.language,
            &wav_bytes,
        )
        .await
    }
}
