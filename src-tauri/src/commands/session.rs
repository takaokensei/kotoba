//! Practice session lifecycle commands.
//!
//! Exposes two Tauri commands:
//! - `start_practice_session` — inserts a fresh `session` row and returns the UUID to the frontend.
//! - `record_session_activity` — called after each attempt to increment word count and
//!   recalculate the rolling average score and elapsed duration.

use sqlx::SqlitePool;
use tauri::State;
use tracing::{info, warn};

use crate::db;

/// Starts a new practice session.
///
/// Inserts a zeroed-out row into the `session` table keyed by a fresh UUID v4.
/// Returns the UUID string to the frontend so it can be passed back on every
/// subsequent `record_session_activity` call.
#[tauri::command]
pub async fn start_practice_session(
    pool: State<'_, SqlitePool>,
) -> Result<String, String> {
    let session_id = db::insert_session(&pool, 0, 0, 0.0)
        .await
        .map_err(|e| e.to_string())?;

    info!(session_id = %session_id, "Practice session started");
    Ok(session_id)
}

/// Records a single attempt result against an active session.
///
/// Delegates to the `record_session_activity` repository function which:
/// - Recomputes `duration_seconds` from `started_at` to *now*.
/// - Increments `words_practiced` by one.
/// - Recalculates the rolling `average_score` as a running mean.
/// - Refreshes the `updated_at` ISO-8601 timestamp.
///
/// Non-fatal: a warning is logged if the session UUID is not found but the
/// command returns `Ok(())` so a missing session does not crash the practice flow.
#[tauri::command]
pub async fn record_session_activity(
    pool: State<'_, SqlitePool>,
    session_id: String,
    score: f64,
) -> Result<(), String> {
    if session_id.is_empty() {
        warn!("record_session_activity: empty session_id — skipping");
        return Ok(());
    }

    db::record_session_activity(&pool, &session_id, score)
        .await
        .map_err(|e| e.to_string())?;

    info!(session_id = %session_id, score, "Session activity recorded");
    Ok(())
}
