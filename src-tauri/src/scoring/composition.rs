use serde::{Deserialize, Serialize};
use std::fs;
use super::levenshtein;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub text: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phonetic: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pgop: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pitch: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ConfigWeights {
    v2: V2Weights,
}

#[derive(Debug, Deserialize)]
struct V2Weights {
    text: f64,
    phonetic: f64,
}

#[derive(Debug, Deserialize)]
struct ScoringConfig {
    weights: ConfigWeights,
}

/// Dynamically pulls weights from scoring_config.json.
/// Falls back to 0.25 (text) and 0.75 (phonetic) if file is missing or invalid.
fn get_v2_weights() -> (f64, f64) {
    let paths = vec![
        std::path::PathBuf::from("scoring_config.json"),
        std::path::PathBuf::from("src-tauri/scoring_config.json"),
    ];

    for path in paths {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(config) = serde_json::from_str::<ScoringConfig>(&content) {
                    return (config.weights.v2.text, config.weights.v2.phonetic);
                }
            }
        }
    }

    // Default fallback weight mapping
    (0.25, 0.75)
}

/// V1 composition: score = text_score scaled to [0, 100].
pub fn compose_v1(target: &str, spoken: &str) -> (f64, ScoreBreakdown, &'static str) {
    let text = levenshtein::normalized_similarity(target, spoken);
    let score = (text * 100.0).clamp(0.0, 100.0);
    let breakdown = ScoreBreakdown {
        text,
        phonetic: None,
        pgop: None,
        pitch: None,
    };
    (score, breakdown, "v1-levenshtein")
}

/// V2 composition: score = text_weight * text_score + phoneme_weight * phoneme_score.
pub fn compose_v2(
    target_text: &str,
    spoken_text: &str,
    target_phonemes: &[String],
    spoken_phonemes: &[String],
    version: &str,
) -> (f64, ScoreBreakdown, String) {
    let text_score = levenshtein::normalized_similarity(target_text, spoken_text);
    let phoneme_score = super::phonemic::normalized_similarity(target_phonemes, spoken_phonemes);

    let (w_text, w_phoneme) = get_v2_weights();
    let score = (w_text * text_score + w_phoneme * phoneme_score) * 100.0;
    let score_clamped = score.clamp(0.0, 100.0);

    let breakdown = ScoreBreakdown {
        text: text_score,
        phonetic: Some(phoneme_score),
        pgop: None,
        pitch: None,
    };

    (score_clamped, breakdown, version.to_string())
}
