use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

use super::catalog::ModelDefinition;
use crate::db;

const MAX_RETRIES: u32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] db::DbError),
    #[error("checksum mismatch after {attempts} attempts")]
    ChecksumMismatch { attempts: u32 },
    #[error("unknown model: {0}")]
    UnknownModel(String),
}

pub type Result<T> = std::result::Result<T, DownloadError>;

pub fn resolve_models_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("kotoba").join("models")
}

pub async fn download_model(
    app: &AppHandle,
    pool: &sqlx::SqlitePool,
    definition: &ModelDefinition,
) -> Result<db::ModelManifestRow> {
    let models_dir = resolve_models_dir();
    std::fs::create_dir_all(&models_dir)?;

    let model_dir = models_dir.join(definition.name);
    std::fs::create_dir_all(&model_dir)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let total_files = definition.files.len();
    let mut primary_path = PathBuf::new();
    let mut primary_checksum = String::new();

    for (index, file) in definition.files.iter().enumerate() {
        let dest = model_dir.join(file.filename);
        let checksum = download_file_with_retry(
            app,
            &client,
            definition.name,
            file.url,
            &dest,
            index,
            total_files,
        )
        .await?;

        if index == 0 {
            primary_path = dest;
            primary_checksum = checksum;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    db::upsert_model_manifest(
        pool,
        definition.name,
        definition.version,
        primary_path.to_string_lossy().as_ref(),
        &primary_checksum,
        &now,
    )
    .await?;

    db::list_model_manifest(pool)
        .await?
        .into_iter()
        .find(|r| r.name == definition.name)
        .ok_or_else(|| DownloadError::UnknownModel(definition.name.to_string()))
}

async fn download_file_with_retry(
    app: &AppHandle,
    client: &reqwest::Client,
    model_name: &str,
    url: &str,
    dest: &Path,
    file_index: usize,
    total_files: usize,
) -> Result<String> {
    let mut last_err: Option<DownloadError> = None;

    for attempt in 1..=MAX_RETRIES {
        match download_once(app, client, model_name, url, dest, file_index, total_files).await {
            Ok(checksum) => return Ok(checksum),
            Err(e) => {
                warn!(
                    model = model_name,
                    url,
                    attempt,
                    error = %e,
                    "download attempt failed"
                );
                let _ = std::fs::remove_file(dest);
                last_err = Some(e);
            }
        }
    }

    Err(last_err.unwrap_or(DownloadError::ChecksumMismatch {
        attempts: MAX_RETRIES,
    }))
}

async fn download_once(
    app: &AppHandle,
    client: &reqwest::Client,
    model_name: &str,
    url: &str,
    dest: &Path,
    file_index: usize,
    total_files: usize,
) -> Result<String> {
    info!(model = model_name, url, "starting download");

    let response = client.get(url).send().await?;
    response.error_for_status_ref()?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut hasher = Sha256::new();

    let mut file = tokio::fs::File::create(dest).await?;
    use tokio::io::AsyncWriteExt;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(DownloadError::Http)?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        let file_fraction = if total > 0 {
            downloaded as f64 / total as f64
        } else {
            0.0
        };
        let overall = ((file_index as f64 + file_fraction) / total_files as f64 * 100.0) as u32;

        let _ = app.emit(
            "model-download-progress",
            serde_json::json!({
                "modelName": model_name,
                "percent": overall.min(99),
            }),
        );
    }

    file.flush().await?;

    let checksum = hex::encode(hasher.finalize());
    info!(model = model_name, path = %dest.display(), checksum = %checksum, "download complete");

    let _ = app.emit(
        "model-download-progress",
        serde_json::json!({
            "modelName": model_name,
            "percent": 100,
        }),
    );

    Ok(checksum)
}
