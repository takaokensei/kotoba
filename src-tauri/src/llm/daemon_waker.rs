//! Tenta acordar o daemon do Ollama antes de declarar indisponibilidade (Seção 15).

use std::process::{Command, Stdio};
use std::time::Duration;

pub async fn ensure_ollama_awake() -> bool {
    if crate::llm::client::is_available().await {
        tracing::info!("Ollama already running");
        return true;
    }

    if try_spawn_ollama_serve() {
        tokio::time::sleep(Duration::from_secs(2)).await;
        if crate::llm::client::is_available().await {
            tracing::info!("Ollama started successfully");
            return true;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
        return crate::llm::client::is_available().await;
    }

    false
}

fn try_spawn_ollama_serve() -> bool {
    tracing::info!("attempting to start `ollama serve` in background");

    #[cfg(windows)]
    {
        let result = Command::new("cmd")
            .args(["/C", "start", "/B", "ollama", "serve"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        return result.is_ok();
    }

    #[cfg(not(windows))]
    {
        let result = Command::new("ollama")
            .arg("serve")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        return result.is_ok();
    }
}

pub async fn ensure_ollama_awake_with_models() -> (bool, Vec<String>) {
    let available = ensure_ollama_awake().await;
    let models = if available {
        crate::llm::client::list_models()
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };
    (available, models)
}
