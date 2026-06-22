//! Sidecar lifecycle manager — whisper.cpp and Piper loaded on demand (ADR Section 7-F).

use std::sync::atomic::{AtomicBool, Ordering};
use std::path::PathBuf;
use tauri::Manager;
use tokio::process::Command as TokioCommand;
use std::process::Stdio;
use std::time::Duration;

static WHISPER_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_whisper_active() -> bool {
    WHISPER_ACTIVE.load(Ordering::SeqCst)
}

pub fn load_whisper() {
    tracing::info!("sidecar lifecycle: loading whisper.cpp");
    WHISPER_ACTIVE.store(true, Ordering::SeqCst);
}

pub fn unload_whisper() {
    tracing::info!("sidecar lifecycle: unloading whisper.cpp");
    WHISPER_ACTIVE.store(false, Ordering::SeqCst);
}

pub fn load_piper() {
    tracing::info!("sidecar lifecycle: loading Piper TTS");
}

pub fn unload_piper() {
    tracing::info!("sidecar lifecycle: unloading Piper TTS");
}

/// Dynamically resolves the sidecar binary path for development and production.
fn resolve_sidecar_path(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    let ext = ".exe";
    #[cfg(not(target_os = "windows"))]
    let ext = "";

    // Resolve target triple
    let target_triple = if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else if cfg!(target_os = "macos") {
        #[cfg(target_arch = "aarch64")]
        { "aarch64-apple-darwin" }
        #[cfg(not(target_arch = "aarch64"))]
        { "x86_64-apple-darwin" }
    } else {
        "x86_64-unknown-linux-gnu"
    };

    let filename = format!("{name}-{target_triple}{ext}");

    // 1. Try relative to current exe (production bundle)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let path = parent.join(&filename);
            if path.exists() {
                return Ok(path);
            }
        }
    }

    // 2. Try development path (relative to cargo workspace root src-tauri)
    let dev_path = PathBuf::from("binaries").join(&filename);
    if dev_path.exists() {
        return Ok(dev_path);
    }

    // 3. Try Tauri resource resolver
    if let Ok(resource_dir) = app.path().resource_dir() {
        let path = resource_dir.join("binaries").join(&filename);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(format!("Não foi possível localizar o sidecar: {filename}"))
}

/// Runs the whisper transcription by launching the whisper.cpp sidecar process.
/// Discards the process immediately after execution (Section 7-F).
pub async fn run_whisper_transcription(
    app: &tauri::AppHandle,
    model_path: &str,
    language: &str,
    wav_bytes: &[u8],
) -> Result<String, String> {
    load_whisper();
    
    let sidecar_path = resolve_sidecar_path(app, "whisper-cli")?;
    tracing::info!(path = %sidecar_path.display(), "Executando sidecar do whisper");
    
    let result = run_transcription_with_tempfile(&sidecar_path, model_path, language, wav_bytes).await;
    
    unload_whisper();
    result
}

async fn run_transcription_with_tempfile(
    sidecar_path: &std::path::Path,
    model_path: &str,
    language: &str,
    wav_bytes: &[u8],
) -> Result<String, String> {
    use std::io::Write as _;
    
    // Create a temporary WAV file using tempfile Builder
    let mut temp_file = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|e| format!("Falha ao criar arquivo temporário de áudio: {e}"))?;
        
    temp_file.write_all(wav_bytes)
        .map_err(|e| format!("Falha ao gravar áudio no arquivo temporário: {e}"))?;
        
    temp_file.flush()
        .map_err(|e| format!("Falha ao descarregar buffer no arquivo temporário: {e}"))?;
        
    let temp_path = temp_file.path().to_path_buf();
    let output_txt_path = std::path::PathBuf::from(format!("{}.txt", temp_path.display()));
    
    let mut child = TokioCommand::new(sidecar_path)
        .arg("--model")
        .arg(model_path)
        .arg("--file")
        .arg(&temp_path)
        .arg("--language")
        .arg(language)
        .arg("--output-txt")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Falha ao iniciar o processo whisper-cli: {e}"))?;
        
    let timeout = Duration::from_secs(5);
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => return Err(format!("Erro ao aguardar whisper-cli: {e}")),
        Err(_) => {
            let _ = child.kill().await;
            return Err("A transcrição do Whisper excedeu o limite de 5s e o processo foi finalizado".to_string());
        }
    };
    
    // The WAV tempfile is deleted from disk here when dropped
    drop(temp_file);
    
    let transcription = if status.success() && output_txt_path.exists() {
        let content = tokio::fs::read_to_string(&output_txt_path).await
            .map_err(|e| format!("Falha ao ler resultado da transcrição: {e}"))?;
        content.trim().to_string()
    } else {
        let mut stderr_bytes = Vec::new();
        if let Some(mut stderr_stream) = child.stderr.take() {
            use tokio::io::AsyncReadExt;
            let _ = stderr_stream.read_to_end(&mut stderr_bytes).await;
        }
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        
        // Clean up the .txt file if it exists (even on error)
        if output_txt_path.exists() {
            let _ = tokio::fs::remove_file(&output_txt_path).await;
        }
        
        return Err(format!("O processo whisper-cli falhou ou não gerou o arquivo de saída. Status: {status:?}. Stderr: {stderr}"));
    };
    
    // Clean up the generated .txt file
    if output_txt_path.exists() {
        let _ = tokio::fs::remove_file(&output_txt_path).await;
    }
    
    Ok(transcription)
}
