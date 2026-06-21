use serde::{Deserialize, Serialize};

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
