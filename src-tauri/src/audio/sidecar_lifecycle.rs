//! Sidecar lifecycle manager — whisper.cpp and Piper loaded on demand (ADR Section 7-F).

use std::sync::atomic::{AtomicBool, Ordering};

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
