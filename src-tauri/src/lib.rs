use std::path::PathBuf;

use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod audio;
mod commands;
mod db;
mod llm;
mod models;
mod scoring;

pub struct AppState {
    pub db_path: PathBuf,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kotoba=debug,tauri=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let db_path = db::resolve_db_path();
            tracing::info!(path = %db_path.display(), "database path resolved");

            let pool = tauri::async_runtime::block_on(async {
                db::init_pool(&db_path).await
            })
            .expect("failed to initialize database");

            app.manage(pool);
            app.manage(AppState { db_path });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::practice::get_next_word,
            commands::practice::score_attempt,
            commands::practice::record_and_transcribe,
            commands::practice::get_tutor_feedback,
            commands::practice::list_recent_attempts,
            commands::onboarding::check_prerequisites,
            commands::onboarding::is_onboarding_required,
            commands::onboarding::list_available_models,
            commands::onboarding::download_model,
            commands::onboarding::get_model_manifest,
            commands::onboarding::check_for_updates,
            commands::onboarding::save_consent,
            commands::tts::speak_word,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::delete_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
