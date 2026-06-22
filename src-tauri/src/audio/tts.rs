use std::path::Path;

pub trait TtsEngine: Send + Sync {
    async fn synthesize(&self, text: &str, output_path: &Path) -> Result<(), String>;
}

pub struct PiperEngine {
    pub app: tauri::AppHandle,
    pub model_path: String,
    pub config_path: String,
}

impl TtsEngine for PiperEngine {
    async fn synthesize(&self, text: &str, output_path: &Path) -> Result<(), String> {
        crate::audio::sidecar_lifecycle::run_piper_tts(
            &self.app,
            &self.model_path,
            &self.config_path,
            text,
            &output_path.to_string_lossy(),
        )
        .await
    }
}
