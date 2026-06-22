//! TTS command — invokes the Piper sidecar and plays the result with rodio.

use sqlx::SqlitePool;
use tauri::{AppHandle, State};

use crate::audio::sidecar_lifecycle::run_piper_tts;
use crate::db;

use std::sync::{Mutex, OnceLock};

static SINK_INSTANCE: OnceLock<Mutex<Option<rodio::MixerDeviceSink>>> = OnceLock::new();

fn get_or_init_sink() -> Result<std::sync::MutexGuard<'static, Option<rodio::MixerDeviceSink>>, String> {
    let mutex = SINK_INSTANCE.get_or_init(|| Mutex::new(None));
    let mut guard = mutex.lock().map_err(|e| format!("Failed to lock audio sink: {e}"))?;
    if guard.is_none() {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink()
            .map_err(|e| format!("No audio output device available: {e}"))?;
        sink.log_on_drop(false);
        *guard = Some(sink);
    }
    Ok(guard)
}

/// Speaks the word identified by `word_id` using Piper TTS.
///
/// Flow:
/// 1. Look up the vocabulary entry (word, reading, language).
/// 2. Determine which Piper voice model corresponds to the language.
/// 3. Synthesise a temporary WAV file via the Piper sidecar.
/// 4. Play it back with `rodio` in a blocking thread.
/// 5. Delete the temp WAV after playback.
#[tauri::command]
pub async fn speak_word(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
    word_id: String,
) -> Result<(), String> {
    // ── 1. Fetch vocabulary entry ────────────────────────────────────────────
    let row = db::get_vocabulary_by_id(&pool, &word_id)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| format!("Word not found: {word_id}"))?;

    // For Japanese, use the phonetic reading if available; fall back to kanji.
    let text_to_speak = if row.language == "ja" {
        row.reading.clone().unwrap_or_else(|| row.word.clone())
    } else {
        row.word.clone()
    };

    // ── 2. Resolve model name and path ───────────────────────────────────────
    let model_name = if row.language == "ja" { "piper-ja" } else { "piper-en" };

    let manifest = db::list_model_manifest(&pool)
        .await
        .map_err(|e| format!("Failed to read model manifest: {e}"))?;

    let model_entry = manifest
        .iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| {
            format!(
                "Piper voice model '{model_name}' not found in manifest. \
                 Please complete the onboarding to download the model."
            )
        })?;

    // model_entry.path is the path to the first downloaded file, which is the .onnx model.
    // (See downloader.rs: primary_path = dest where dest = model_dir.join(file.filename))
    let onnx_path = std::path::PathBuf::from(&model_entry.path);

    if !onnx_path.exists() {
        return Err(format!(
            "Model file not found: {:?}. Please re-run the onboarding to re-download the model.",
            onnx_path
        ));
    }

    // The JSON config lives alongside the .onnx with a `.onnx.json` extension.
    let onnx_filename = onnx_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("model.onnx")
        .to_string();
    let config_path = onnx_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join(format!("{onnx_filename}.json"));

    // ── 3. Create temp WAV file and release the handle ───────────────────────
    let temp_file = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|e| format!("Failed to create temp WAV file: {e}"))?;

    let (file, temp_path) = temp_file.into_parts();
    // Close the file handle immediately so Piper can open and write to the path.
    drop(file);

    let wav_path_str = temp_path.to_string_lossy().to_string();

    // ── 4. Run Piper sidecar ─────────────────────────────────────────────────
    run_piper_tts(
        &app,
        &onnx_path.to_string_lossy(),
        &config_path.to_string_lossy(),
        &text_to_speak,
        &wav_path_str,
    )
    .await?;

    // ── 5. Play WAV with rodio in a blocking thread ──────────────────────────
    // Move the TempPath into the thread so it is deleted after playback.
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::open(&*temp_path)
            .map_err(|e| format!("Cannot open generated WAV: {e}"))?;

        // Get or initialize the global persistent DeviceSink
        let sink_guard = get_or_init_sink()?;
        let sink_ref = sink_guard.as_ref().unwrap();

        use rodio::Source as _;

        // Decode the WAV file into samples
        let decoder = rodio::Decoder::try_from(file)
            .map_err(|e| format!("Failed to decode WAV: {e}"))?;

        let channels = decoder.channels();
        let sample_rate = decoder.sample_rate();

        let channels_u16 = channels.get();
        let sample_rate_u32 = sample_rate.get();

        // Precompute 400ms (0.40s) pre-roll silence buffer (zero-valued samples)
        // Extract primitive values from NonZero types for casting
        let silence_samples_count = ((sample_rate_u32 as f32) * (channels_u16 as f32) * 0.40) as usize;
        let mut all_samples = vec![0.0f32; silence_samples_count];

        // Collect all decoded samples and append them to silence pre-roll
        let decoded_samples: Vec<f32> = decoder.collect();
        all_samples.extend(decoded_samples);

        // Construct a SamplesBuffer
        let source = rodio::buffer::SamplesBuffer::new(channels, sample_rate, all_samples);

        // Connect player to persistent mixer
        let player = rodio::Player::connect_new(sink_ref.mixer());

        // Queue source and start playback
        player.append(source);

        player.sleep_until_end();

        // Give the driver time to flush the remaining hardware buffer to speakers
        std::thread::sleep(std::time::Duration::from_millis(300));

        // temp_path is dropped here → temp file deleted from disk
        drop(temp_path);
        Ok(())
    })
    .await
    .map_err(|e| format!("Playback thread panicked: {e}"))??;

    Ok(())
}
