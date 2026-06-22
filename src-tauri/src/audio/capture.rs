//! Microphone capture — implemented in Sprint 1 Task 1.3.

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use tauri::Listener;

static LAST_RECORDED_WAV: Mutex<Option<Vec<u8>>> = Mutex::new(None);

pub fn set_last_recorded_wav(wav: Vec<u8>) {
    let mut lock = LAST_RECORDED_WAV.lock().unwrap();
    *lock = Some(wav);
}

pub fn take_last_recorded_wav() -> Option<Vec<u8>> {
    let mut lock = LAST_RECORDED_WAV.lock().unwrap();
    lock.take()
}

pub fn clear_last_recorded_wav() {
    let mut lock = LAST_RECORDED_WAV.lock().unwrap();
    *lock = None;
}

/// Helper function to create a 44-byte WAV header for 16kHz, 16-bit, mono PCM.
fn create_wav_header(data_len: usize) -> [u8; 44] {
    let mut header = [0u8; 44];
    
    // RIFF identifier
    header[0..4].copy_from_slice(b"RIFF");
    
    // File size - 8
    let file_size = (36 + data_len) as u32;
    header[4..8].copy_from_slice(&file_size.to_le_bytes());
    
    // WAVE identifier
    header[8..12].copy_from_slice(b"WAVE");
    
    // fmt subchunk identifier
    header[12..16].copy_from_slice(b"fmt ");
    
    // fmt subchunk size (16 for PCM)
    let fmt_size = 16u32;
    header[16..20].copy_from_slice(&fmt_size.to_le_bytes());
    
    // Audio format (1 for PCM)
    let audio_format = 1u16;
    header[20..22].copy_from_slice(&audio_format.to_le_bytes());
    
    // Number of channels (1 for mono)
    let num_channels = 1u16;
    header[22..24].copy_from_slice(&num_channels.to_le_bytes());
    
    // Sample rate (16000 Hz)
    let sample_rate = 16000u32;
    header[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    
    // Byte rate (sample_rate * num_channels * bits_per_sample / 8) = 16000 * 1 * 16 / 8 = 32000
    let byte_rate = 32000u32;
    header[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    
    // Block align (num_channels * bits_per_sample / 8) = 1 * 16 / 8 = 2
    let block_align = 2u16;
    header[32..34].copy_from_slice(&block_align.to_le_bytes());
    
    // Bits per sample (16)
    let bits_per_sample = 16u16;
    header[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
    
    // data subchunk identifier
    header[36..40].copy_from_slice(b"data");
    
    // data subchunk size
    let data_size = data_len as u32;
    header[40..44].copy_from_slice(&data_size.to_le_bytes());
    
    header
}

/// Convert multi-channel samples to mono by averaging channels.
fn convert_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    
    let channels = channels as usize;
    let mut mono = Vec::with_capacity(samples.len() / channels);
    for chunk in samples.chunks_exact(channels) {
        let sum: f32 = chunk.iter().sum();
        mono.push(sum / channels as f32);
    }
    mono
}

/// Linear interpolation resampler from from_rate to to_rate.
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }
    
    let ratio = from_rate as f64 / to_rate as f64;
    let new_len = (samples.len() as f64 / ratio).floor() as usize;
    let mut resampled = Vec::with_capacity(new_len);
    
    for i in 0..new_len {
        let pos = i as f64 * ratio;
        let idx = pos.floor() as usize;
        let frac = pos - idx as f64;
        
        if idx + 1 < samples.len() {
            let s1 = samples[idx];
            let s2 = samples[idx + 1];
            resampled.push(s1 + (s2 - s1) * frac as f32);
        } else if idx < samples.len() {
            resampled.push(samples[idx]);
        }
    }
    
    resampled
}

/// Convert f32 samples in range [-1.0, 1.0] to i16 range [-32768, 32767].
fn convert_to_s16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|&s| {
        let clamped = s.clamp(-1.0, 1.0);
        let scaled = clamped * 32767.0;
        scaled as i16
    }).collect()
}

/// Real-time capture of microphone audio.
/// Listens to stop-recording and cancel-recording events from frontend.
/// Returns the resampled 16kHz WAV bytes.
pub async fn capture_mic_audio(
    app_handle: &tauri::AppHandle,
    max_duration_ms: u64,
) -> Result<Option<Vec<u8>>, String> {
    use tokio::sync::oneshot;
    
    let (tx, rx) = oneshot::channel();
    let app_handle_clone = app_handle.clone();
    
    std::thread::spawn(move || {
        let result = capture_mic_audio_sync(&app_handle_clone, max_duration_ms);
        let _ = tx.send(result);
    });
    
    match rx.await {
        Ok(result) => result,
        Err(_) => Err("A thread de gravação de áudio falhou inesperadamente".to_string()),
    }
}

fn capture_mic_audio_sync(
    app_handle: &tauri::AppHandle,
    max_duration_ms: u64,
) -> Result<Option<Vec<u8>>, String> {
    // Get default input host and device
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "Nenhum dispositivo de entrada de áudio encontrado".to_string())?;
    
    let config = device
        .default_input_config()
        .map_err(|e| format!("Falha ao obter configuração do microfone: {e}"))?;
    
    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
    
    let recording_buffer = Arc::new(Mutex::new(Vec::new()));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag = Arc::new(AtomicBool::new(false));
    
    // Set up event listeners
    let stop_flag_clone = Arc::clone(&stop_flag);
    let stop_listener = app_handle.listen("stop-recording", move |_event| {
        stop_flag_clone.store(true, Ordering::SeqCst);
    });
    
    let cancel_flag_clone = Arc::clone(&cancel_flag);
    let cancel_listener = app_handle.listen("cancel-recording", move |_event| {
        cancel_flag_clone.store(true, Ordering::SeqCst);
    });
    
    // Build and start the CPAL stream
    let recording_buffer_clone = Arc::clone(&recording_buffer);
    let err_handler = |err| {
        tracing::error!("Erro no fluxo de áudio cpal: {err}");
    };
    
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if let Ok(mut buf) = recording_buffer_clone.lock() {
                        buf.extend_from_slice(data);
                    }
                },
                err_handler,
                None
            )
        }
        cpal::SampleFormat::I16 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if let Ok(mut buf) = recording_buffer_clone.lock() {
                        buf.extend(data.iter().map(|&s| s as f32 / 32768.0));
                    }
                },
                err_handler,
                None
            )
        }
        cpal::SampleFormat::U16 => {
            device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| {
                    if let Ok(mut buf) = recording_buffer_clone.lock() {
                        buf.extend(data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0));
                    }
                },
                err_handler,
                None
            )
        }
        _ => return Err("Formato de amostra de áudio não suportado".to_string()),
    }.map_err(|e| format!("Falha ao construir fluxo de gravação: {e}"))?;
    
    stream.play().map_err(|e| format!("Falha ao iniciar gravação: {e}"))?;
    
    let start_time = Instant::now();
    let max_duration = Duration::from_millis(max_duration_ms);
    
    // Loop until stopped, cancelled, or max duration reached
    while !stop_flag.load(Ordering::SeqCst) && !cancel_flag.load(Ordering::SeqCst) {
        if start_time.elapsed() >= max_duration {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    
    // Stop the stream and cleanup listeners
    let _ = stream.pause();
    app_handle.unlisten(stop_listener);
    app_handle.unlisten(cancel_listener);
    
    if cancel_flag.load(Ordering::SeqCst) {
        return Ok(None);
    }
    
    // Retrieve recorded samples
    let raw_samples = {
        let lock = recording_buffer.lock().unwrap();
        lock.clone()
    };
    
    if raw_samples.is_empty() {
        return Ok(None);
    }
    
    // Process audio: mono, resample to 16000Hz, convert to s16
    let mono_samples = convert_to_mono(&raw_samples, channels);
    let resampled_samples = resample(&mono_samples, sample_rate, 16000);
    let s16_samples = convert_to_s16(&resampled_samples);
    
    // Format to WAV file in-memory
    let wav_data: Vec<u8> = s16_samples.iter().flat_map(|&s| s.to_le_bytes().to_vec()).collect();
    let header = create_wav_header(wav_data.len());
    let mut wav_bytes = Vec::with_capacity(44 + wav_data.len());
    wav_bytes.extend_from_slice(&header);
    wav_bytes.extend_from_slice(&wav_data);
    
    Ok(Some(wav_bytes))
}
