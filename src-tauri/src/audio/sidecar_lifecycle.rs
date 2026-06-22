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
    wav_bytes: &[u8],
) -> Result<String, String> {
    load_whisper();
    
    let sidecar_path = resolve_sidecar_path(app, "whisper-cli")?;
    tracing::info!(path = %sidecar_path.display(), "Executando sidecar do whisper");
    
    let result = run_transcription_internal(&sidecar_path, model_path, wav_bytes).await;
    
    unload_whisper();
    result
}

async fn run_transcription_internal(
    sidecar_path: &std::path::Path,
    model_path: &str,
    wav_bytes: &[u8],
) -> Result<String, String> {
    // 1. Try passing WAV bytes to whisper-cli via stdin
    let mut child = TokioCommand::new(sidecar_path)
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg("-")
        .arg("-nt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Falha ao iniciar o processo whisper-cli: {e}"))?;
        
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        if let Err(e) = stdin.write_all(wav_bytes).await {
            tracing::warn!("Falha ao escrever no pipe stdin, tentando com arquivo temporário: {e}");
            return run_transcription_with_tempfile(sidecar_path, model_path, wav_bytes).await;
        }
    }
    
    let mut stdout_stream = child.stdout.take().ok_or("Falha ao abrir stdout do whisper-cli")?;
    let mut stderr_stream = child.stderr.take().ok_or("Falha ao abrir stderr do whisper-cli")?;
    
    let timeout = Duration::from_secs(5);
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => return Err(format!("Erro ao aguardar whisper-cli: {e}")),
        Err(_) => {
            let _ = child.kill().await;
            return Err("A transcrição do Whisper excedeu o limite de 5s e o processo foi finalizado".to_string());
        }
    };
    
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    use tokio::io::AsyncReadExt as _;
    let _ = stdout_stream.read_to_end(&mut stdout_bytes).await;
    let _ = stderr_stream.read_to_end(&mut stderr_bytes).await;
    
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        tracing::warn!("whisper-cli retornou erro: {stderr}. Tentando fallback com arquivo temporário.");
        return run_transcription_with_tempfile(sidecar_path, model_path, wav_bytes).await;
    }
    
    let transcription = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
    Ok(transcription)
}

async fn run_transcription_with_tempfile(
    sidecar_path: &std::path::Path,
    model_path: &str,
    wav_bytes: &[u8],
) -> Result<String, String> {
    use std::io::Write as _;
    
    // Create a temporary file that is automatically deleted when dropped
    let mut temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| format!("Falha ao criar arquivo temporário de áudio: {e}"))?;
        
    temp_file.write_all(wav_bytes)
        .map_err(|e| format!("Falha ao gravar áudio no arquivo temporário: {e}"))?;
        
    let temp_path = temp_file.path().to_path_buf();
    
    let mut child = TokioCommand::new(sidecar_path)
        .arg("-m")
        .arg(model_path)
        .arg("-f")
        .arg(&temp_path)
        .arg("-nt")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Falha ao iniciar o processo whisper-cli com arquivo temporário: {e}"))?;
        
    let mut stdout_stream = child.stdout.take().ok_or("Falha ao abrir stdout do whisper-cli")?;
    let mut stderr_stream = child.stderr.take().ok_or("Falha ao abrir stderr do whisper-cli")?;
    
    let timeout = Duration::from_secs(5);
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => return Err(format!("Erro ao aguardar whisper-cli: {e}")),
        Err(_) => {
            let _ = child.kill().await;
            return Err("A transcrição com arquivo temporário excedeu 5s e o processo foi finalizado".to_string());
        }
    };
    
    // The tempfile is deleted from disk here when dropped
    drop(temp_file);
    
    let mut stdout_bytes = Vec::new();
    let mut stderr_bytes = Vec::new();
    use tokio::io::AsyncReadExt as _;
    let _ = stdout_stream.read_to_end(&mut stdout_bytes).await;
    let _ = stderr_stream.read_to_end(&mut stderr_bytes).await;
    
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        return Err(format!("O processo whisper-cli falhou: {stderr}"));
    }
    
    let transcription = String::from_utf8_lossy(&stdout_bytes).trim().to_string();
    Ok(transcription)
}
