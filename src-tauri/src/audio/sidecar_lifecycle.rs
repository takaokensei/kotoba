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

/// Runs Piper TTS to synthesise `text`, writing a WAV file to `output_wav_path`.
/// Resolves Windows 8.3 short paths for both the model and output file to prevent
/// crashes on non-ASCII paths (e.g. paths containing accented characters).
pub async fn run_piper_tts(
    app: &tauri::AppHandle,
    model_path: &str,
    config_path: &str,
    text: &str,
    output_wav_path: &str,
) -> Result<(), String> {
    load_piper();

    let sidecar_path = resolve_sidecar_path(app, "piper")?;
    tracing::info!(path = %sidecar_path.display(), "Executando sidecar do Piper");

    let model_path_resolved = if cfg!(target_os = "windows") {
        get_short_path(model_path).unwrap_or_else(|| model_path.to_string())
    } else {
        model_path.to_string()
    };

    let config_path_resolved = if cfg!(target_os = "windows") {
        get_short_path(config_path).unwrap_or_else(|| config_path.to_string())
    } else {
        config_path.to_string()
    };

    let output_path_resolved = if cfg!(target_os = "windows") {
        get_short_path(output_wav_path).unwrap_or_else(|| output_wav_path.to_string())
    } else {
        output_wav_path.to_string()
    };

    // Resolve the espeak-ng-data directory that lives next to the sidecar binary
    let espeak_data_path = sidecar_path
        .parent()
        .map(|p| p.join("espeak-ng-data"))
        .unwrap_or_else(|| std::path::PathBuf::from("espeak-ng-data"));

    let espeak_data_resolved = if cfg!(target_os = "windows") {
        get_short_path(&espeak_data_path.to_string_lossy())
            .unwrap_or_else(|| espeak_data_path.to_string_lossy().to_string())
    } else {
        espeak_data_path.to_string_lossy().to_string()
    };

    use tokio::io::AsyncWriteExt as _;

    let mut child = TokioCommand::new(&sidecar_path)
        .arg("--model")
        .arg(&model_path_resolved)
        .arg("--config")
        .arg(&config_path_resolved)
        .arg("--output_file")
        .arg(&output_path_resolved)
        .arg("--espeak-data")
        .arg(&espeak_data_resolved)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Falha ao iniciar o processo piper: {e}"))?;

    // Feed text into Piper's stdin then close the pipe
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|e| format!("Falha ao escrever texto no stdin do Piper: {e}"))?;
        // stdin is dropped here, signalling EOF to the process
    }

    let timeout = Duration::from_secs(30);
    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(e)) => {
            unload_piper();
            return Err(format!("Erro ao aguardar piper: {e}"));
        }
        Err(_) => {
            let _ = child.kill().await;
            unload_piper();
            return Err("O processo Piper TTS excedeu o limite de 30s e foi finalizado".to_string());
        }
    };

    if !status.success() {
        let mut stderr_bytes = Vec::new();
        if let Some(mut stderr_stream) = child.stderr.take() {
            use tokio::io::AsyncReadExt;
            let _ = stderr_stream.read_to_end(&mut stderr_bytes).await;
        }
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        unload_piper();
        return Err(format!(
            "O processo Piper falhou. Status: {status:?}. Stderr: {stderr}"
        ));
    }

    unload_piper();
    Ok(())
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
        
    // Decouple file handle and path wrapper
    let (file, temp_path) = temp_file.into_parts();
    // Drop the file handle immediately to close and release the OS file lock on Windows
    drop(file);
    
    // Resolve short 8.3 paths on Windows to bypass non-ASCII path issues in whisper.cpp
    let model_path_str = if cfg!(target_os = "windows") {
        get_short_path(model_path).unwrap_or_else(|| model_path.to_string())
    } else {
        model_path.to_string()
    };

    let temp_path_str = if cfg!(target_os = "windows") {
        get_short_path(&temp_path.to_string_lossy()).unwrap_or_else(|| temp_path.to_string_lossy().to_string())
    } else {
        temp_path.to_string_lossy().to_string()
    };
    
    let output_txt_path = std::path::PathBuf::from(format!("{}.txt", temp_path_str));
    
    let mut child = TokioCommand::new(sidecar_path)
        .arg("--model")
        .arg(&model_path_str)
        .arg("--file")
        .arg(&temp_path_str)
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
        Ok(Err(e)) => {
            // Delete the WAV file if wait fails
            drop(temp_path);
            return Err(format!("Erro ao aguardar whisper-cli: {e}"));
        }
        Err(_) => {
            let _ = child.kill().await;
            // Delete the WAV file if timeout occurs
            drop(temp_path);
            return Err("A transcrição do Whisper excedeu o limite de 5s e o processo foi finalizado".to_string());
        }
    };
    
    // The WAV tempfile is deleted from disk here when TempPath is dropped
    drop(temp_path);
    
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

#[cfg(target_os = "windows")]
extern "system" {
    fn GetShortPathNameW(
        lpszLongPath: *const u16,
        lpszShortPath: *mut u16,
        cchBuffer: u32,
    ) -> u32;
}

#[cfg(target_os = "windows")]
fn get_short_path(long_path: &str) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    
    let wide_path: Vec<u16> = OsStr::new(long_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
        
    unsafe {
        let len = GetShortPathNameW(wide_path.as_ptr(), std::ptr::null_mut(), 0);
        if len == 0 {
            return None;
        }
        
        let mut buffer = vec![0u16; len as usize];
        let res = GetShortPathNameW(wide_path.as_ptr(), buffer.as_mut_ptr(), len);
        if res != 0 {
            Some(String::from_utf16_lossy(&buffer[..res as usize]))
        } else {
            None
        }
    }
}
