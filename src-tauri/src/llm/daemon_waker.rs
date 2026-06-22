//! Ollama Daemon Waker.
//!
//! Tries to wake up the host's local Ollama service in the background upon
//! connection failure, avoiding friction for non-technical users.

use std::process::{Command, Stdio};
use std::time::Duration;

/// Returns true if the `ollama` executable is present in the system's `PATH`.
pub fn is_ollama_in_path() -> bool {
    Command::new("ollama")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

/// Detaches a child process executing `ollama serve` in the background (non-blocking).
fn try_spawn_ollama_serve() -> bool {
    tracing::info!("attempting to start `ollama serve` in background");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let result = Command::new("ollama")
            .arg("serve")
            .creation_flags(CREATE_NO_WINDOW)
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

/// Automatically wake up the host's local Ollama service if offline.
pub async fn ensure_ollama_awake() -> bool {
    if crate::llm::client::is_available().await {
        tracing::info!("Ollama already running");
        return true;
    }

    if !is_ollama_in_path() {
        tracing::warn!("Ollama executable not found in PATH; cannot auto-wake daemon");
        return false;
    }

    if try_spawn_ollama_serve() {
        // Short loop with an exponential backoff retry system (max 3-4 seconds total)
        // Delay sequence: 100ms, 200ms, 400ms, 800ms, 1600ms = 3.1s total wait
        let mut delay = Duration::from_millis(100);
        let max_wait = Duration::from_secs(4);
        let start_time = std::time::Instant::now();

        while start_time.elapsed() < max_wait {
            if crate::llm::client::is_available().await {
                tracing::info!("Ollama started successfully after background wake-up");
                return true;
            }
            tokio::time::sleep(delay).await;
            delay *= 2;
        }

        // Final check
        if crate::llm::client::is_available().await {
            tracing::info!("Ollama started successfully (final waker check)");
            return true;
        }
    }

    tracing::error!("Ollama serve was spawned, but did not respond in time");
    false
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
